// Plugin webview lifecycle and bounds.
use super::*;

pub(super) fn open_plugin_webview(
    app: &tauri::AppHandle,
    plugin_id: &str,
    project_id: &str,
    host: &PluginHost,
    view_id: Option<&str>,
) -> Result<(), String> {
    let entry = host
        .runtime_entry(project_id, plugin_id)
        .ok_or_else(|| "plugin is not installed".to_string())?;
    if entry.manifest.kind != daena_plugin_api::PluginKind::Sandboxed {
        return Err("only sandboxed plugins have UI webviews".into());
    }
    if host.lifecycle.state(project_id, plugin_id).state != daena_plugin_api::LifecycleState::Active
    {
        return Err("plugin is not active".into());
    }
    let policy =
        webview_policy(&entry.manifest).ok_or_else(|| "plugin has no UI entrypoint".to_string())?;
    validate_plugin_view(&entry.manifest, view_id)?;
    let host_surface = host_surface_for_view(&entry.manifest, view_id);
    if let Some(window) = app.get_webview_window(&policy.label) {
        window.show().map_err(|error| error.to_string())?;
        window.set_focus().map_err(|error| error.to_string())?;
        return Ok(());
    }
    let mut url = format!("{}?project={}", policy.url, percent_encode(project_id));
    append_host_surface_query(&mut url, host_surface);
    if let Some(view_id) = view_id {
        url.push_str("&view=");
        url.push_str(&percent_encode(view_id));
    }
    let navigation_policy = policy.clone();
    tauri::WebviewWindowBuilder::new(
        app,
        policy.label,
        tauri::WebviewUrl::External(
            url.parse()
                .map_err(|error| format!("invalid plugin URL: {error}"))?,
        ),
    )
    .use_https_scheme(true)
    .initialization_script(PLUGIN_WEBVIEW_ISOLATION_SCRIPT)
    .on_navigation(move |url| plugin_navigation_allowed(url, &navigation_policy))
    .title(entry.manifest.name.clone())
    .inner_size(980.0, 720.0)
    .visible(true)
    .build()
    .map_err(|error| error.to_string())?;
    Ok(())
}

pub(super) fn host_surface_for_view<'a>(
    manifest: &'a PluginManifest,
    view_id: Option<&str>,
) -> Option<(&'a str, u32)> {
    let view = view_id
        .and_then(|id| manifest.views.iter().find(|view| view.id == id))
        .or_else(|| manifest.views.first())?;
    match &view.renderer {
        daena_plugin_api::ViewRenderer::HostSurface { id, major } => Some((id.as_str(), *major)),
        _ => None,
    }
}

pub(super) fn append_host_surface_query(url: &mut String, host_surface: Option<(&str, u32)>) {
    let Some((id, major)) = host_surface else {
        return;
    };
    url.push_str("&hostSurface=");
    url.push_str(&percent_encode(id));
    url.push_str("&hostSurfaceMajor=");
    url.push_str(&major.to_string());
}

pub(super) fn close_plugin_webview(app: &tauri::AppHandle, plugin_id: &str) {
    let label = plugin_window_label(plugin_id);
    if let Ok(mut states) = embedded_webview_states().lock() {
        states.remove(&label);
    }
    if let Some(webview) = app.get_webview(&label) {
        let _ = webview.close();
    }
    if let Some(window) = app.get_webview_window(&label) {
        let _ = window.close();
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PluginWebviewBounds {
    pub(super) x: f64,
    pub(super) y: f64,
    pub(super) width: f64,
    pub(super) height: f64,
    #[serde(rename = "viewportWidth")]
    pub(super) viewport_width: f64,
    #[serde(rename = "viewportHeight")]
    pub(super) viewport_height: f64,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct EmbeddedPluginWebviewState {
    pub(super) bounds: PluginWebviewBounds,
    pub(super) ready: bool,
}

pub(super) fn embedded_webview_states(
) -> &'static Mutex<BTreeMap<String, EmbeddedPluginWebviewState>> {
    static STATES: OnceLock<Mutex<BTreeMap<String, EmbeddedPluginWebviewState>>> = OnceLock::new();
    STATES.get_or_init(|| Mutex::new(BTreeMap::new()))
}

impl PluginWebviewBounds {
    pub(super) fn validate(&self) -> Result<(), String> {
        if !self.x.is_finite()
            || !self.y.is_finite()
            || !self.width.is_finite()
            || !self.height.is_finite()
            || !self.viewport_width.is_finite()
            || !self.viewport_height.is_finite()
            || self.x < 0.0
            || self.y < 0.0
            || !(1.0..=10_000.0).contains(&self.width)
            || !(1.0..=10_000.0).contains(&self.height)
            || !(1.0..=10_000.0).contains(&self.viewport_width)
            || !(1.0..=10_000.0).contains(&self.viewport_height)
        {
            return Err("plugin webview bounds are invalid".into());
        }
        Ok(())
    }
}

pub(super) fn scale_plugin_bounds(
    bounds: PluginWebviewBounds,
    native_viewport_width: f64,
    native_viewport_height: f64,
) -> PluginWebviewBounds {
    let scale_x = native_viewport_width / bounds.viewport_width;
    let scale_y = native_viewport_height / bounds.viewport_height;
    PluginWebviewBounds {
        x: bounds.x * scale_x,
        y: bounds.y * scale_y,
        width: bounds.width * scale_x,
        height: bounds.height * scale_y,
        viewport_width: native_viewport_width,
        viewport_height: native_viewport_height,
    }
}

pub(super) fn native_plugin_bounds(
    app: &tauri::AppHandle,
    bounds: PluginWebviewBounds,
) -> Result<PluginWebviewBounds, String> {
    bounds.validate()?;
    let main = app
        .get_window("main")
        .ok_or_else(|| "main window is not available".to_string())?;
    let scale_factor = main.scale_factor().map_err(|error| error.to_string())?;
    let native_size = main
        .inner_size()
        .map_err(|error| error.to_string())?
        .to_logical::<f64>(scale_factor);
    // `add_child` attaches the plugin webview to the main window's content
    // view. The browser rectangle is also measured from that content view,
    // so adding the title-bar/frame inset here would shift the child down a
    // second time. Keep the coordinates in content-view space and only
    // normalize for a scale-factor/viewport change.
    let native_bounds = scale_plugin_bounds(bounds, native_size.width, native_size.height);
    native_bounds.validate()?;
    Ok(native_bounds)
}

pub(super) fn plugin_webview_url(
    policy: &daena_plugin_host::PluginWebviewPolicy,
    project_id: &str,
    view_id: Option<&str>,
    host_surface: Option<(&str, u32)>,
    map_entity_id: Option<&str>,
    link_id: Option<&str>,
    bounds: PluginWebviewBounds,
) -> Result<tauri::WebviewUrl, String> {
    let mut url = format!(
        "{}?project={}&width={}&height={}",
        policy.url,
        percent_encode(project_id),
        bounds.width,
        bounds.height
    );
    append_host_surface_query(&mut url, host_surface);
    if let Some(view_id) = view_id {
        url.push_str("&view=");
        url.push_str(&percent_encode(view_id));
    }
    if let Some(map_entity_id) = map_entity_id {
        url.push_str("&mapEntityId=");
        url.push_str(&percent_encode(map_entity_id));
    }
    if let Some(link_id) = link_id {
        url.push_str("&linkId=");
        url.push_str(&percent_encode(link_id));
    }
    Ok(tauri::WebviewUrl::External(
        url.parse()
            .map_err(|error| format!("invalid plugin URL: {error}"))?,
    ))
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub(super) async fn plugin_mount_webview(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedCore>,
    plugins: tauri::State<'_, SharedPluginHost>,
    plugin_id: String,
    view_id: Option<String>,
    map_entity_id: Option<String>,
    link_id: Option<String>,
    bounds: PluginWebviewBounds,
) -> Result<(), String> {
    let bounds = native_plugin_bounds(&app, bounds)?;
    let project_id = current_info(state.inner())?
        .map(|info| info.root)
        .ok_or_else(|| "project is not open".to_string())?;
    let (policy, url) = {
        let host = plugins
            .lock()
            .map_err(|_| "plugin host lock poisoned".to_string())?;
        let entry = host
            .runtime_entry(&project_id, &plugin_id)
            .ok_or_else(|| "plugin is not installed".to_string())?;
        if entry.manifest.kind != daena_plugin_api::PluginKind::Sandboxed {
            return Err("only sandboxed plugins have UI webviews".into());
        }
        if host.lifecycle.state(&project_id, &plugin_id).state
            != daena_plugin_api::LifecycleState::Active
        {
            return Err("plugin is not active".into());
        }
        let policy = webview_policy(&entry.manifest)
            .ok_or_else(|| "plugin has no UI entrypoint".to_string())?;
        validate_plugin_view(&entry.manifest, view_id.as_deref())?;
        let host_surface = host_surface_for_view(&entry.manifest, view_id.as_deref());
        let url = plugin_webview_url(
            &policy,
            &project_id,
            view_id.as_deref(),
            host_surface,
            map_entity_id.as_deref(),
            link_id.as_deref(),
            bounds,
        )?;
        (policy, url)
    };
    let label = policy.label.clone();

    // Only one embedded plugin view occupies the workspace. Do not close a
    // legacy external plugin window with the same identity.
    for (other_label, webview) in app.webviews() {
        if other_label.starts_with("plugin:")
            && other_label != label
            && app.get_webview_window(&other_label).is_none()
        {
            if let Ok(mut states) = embedded_webview_states().lock() {
                states.remove(&other_label);
            }
            let _ = webview.close();
        }
    }

    // Always recreate the embedded webview on mount. Reusing an existing
    // child and only resizing it drops query params such as mapEntityId, so
    // opening a saved map (or switching maps) would keep the previous source
    // loaded and never hit asset.read / uploadMap for the requested entity.
    if let Some(window) = app.get_webview_window(&label) {
        window.close().map_err(|error| error.to_string())?;
    }
    if let Some(webview) = app.get_webview(&label) {
        if let Ok(mut states) = embedded_webview_states().lock() {
            states.remove(&label);
        }
        webview.close().map_err(|error| error.to_string())?;
    }

    let navigation_policy = policy.clone();
    let main = app
        .get_window("main")
        .ok_or_else(|| "main window is not available".to_string())?;
    let builder = tauri::WebviewBuilder::new(label.clone(), url)
        .use_https_scheme(true)
        .initialization_script(PLUGIN_WEBVIEW_ISOLATION_SCRIPT)
        .on_navigation(move |url| plugin_navigation_allowed(url, &navigation_policy))
        .on_page_load(move |webview, payload| {
            if matches!(payload.event(), tauri::webview::PageLoadEvent::Finished) {
                let bounds = embedded_webview_states()
                    .lock()
                    .ok()
                    .and_then(|mut states| {
                        let state = states.get_mut(webview.label())?;
                        state.ready = true;
                        Some(state.bounds)
                    });
                if let Some(bounds) = bounds {
                    let _ = webview.set_position(tauri::LogicalPosition::new(bounds.x, bounds.y));
                    let _ = webview.set_size(tauri::LogicalSize::new(bounds.width, bounds.height));
                    let _ = webview.show();
                }
            }
        });
    // Keep the child effectively invisible until its first document has
    // painted. Showing a newly-created native webview at its final bounds can
    // expose its platform background for a frame, which appears as a flash.
    {
        let mut states = embedded_webview_states()
            .lock()
            .map_err(|_| "embedded webview state lock poisoned".to_string())?;
        states.insert(
            label.clone(),
            EmbeddedPluginWebviewState {
                bounds,
                ready: false,
            },
        );
    }
    if let Err(error) = main.add_child(
        builder,
        tauri::LogicalPosition::new(0.0, 0.0),
        tauri::LogicalSize::new(1.0, 1.0),
    ) {
        if let Ok(mut states) = embedded_webview_states().lock() {
            states.remove(&label);
        }
        return Err(error.to_string());
    }
    Ok(())
}

#[tauri::command]
pub(super) async fn plugin_resize_webview(
    app: tauri::AppHandle,
    bounds: PluginWebviewBounds,
    plugin_id: String,
) -> Result<(), String> {
    let bounds = native_plugin_bounds(&app, bounds)?;
    let label = plugin_window_label(&plugin_id);
    let webview = app
        .get_webview(&label)
        .ok_or_else(|| "embedded plugin webview is not mounted".to_string())?;
    let ready = embedded_webview_states()
        .lock()
        .map(|mut states| {
            let state = states.entry(label).or_insert(EmbeddedPluginWebviewState {
                bounds,
                ready: true,
            });
            state.bounds = bounds;
            state.ready
        })
        .map_err(|_| "embedded webview state lock poisoned".to_string())?;
    if !ready {
        return Ok(());
    }
    webview
        .set_position(tauri::LogicalPosition::new(bounds.x, bounds.y))
        .map_err(|error| error.to_string())?;
    webview
        .set_size(tauri::LogicalSize::new(bounds.width, bounds.height))
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
pub(super) fn plugin_unmount_webview(
    app: tauri::AppHandle,
    plugin_id: String,
) -> Result<(), String> {
    let label = plugin_window_label(&plugin_id);
    if let Ok(mut states) = embedded_webview_states().lock() {
        states.remove(&label);
    }
    if let Some(webview) = app.get_webview(&label) {
        if app.get_webview_window(&label).is_none() {
            webview.close().map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

pub(super) fn validate_plugin_view(
    manifest: &PluginManifest,
    view_id: Option<&str>,
) -> Result<(), String> {
    let Some(view_id) = view_id else {
        return Ok(());
    };
    if manifest.views.iter().any(|view| view.id == view_id) {
        Ok(())
    } else {
        Err(format!("plugin view is not declared: {view_id}"))
    }
}

#[tauri::command]
pub(super) fn plugin_open_webview(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedCore>,
    plugins: tauri::State<'_, SharedPluginHost>,
    plugin_id: String,
    view_id: Option<String>,
) -> Result<(), String> {
    let project_id = current_info(state.inner())?
        .map(|info| info.root)
        .ok_or_else(|| "project is not open".to_string())?;
    let host = plugins
        .lock()
        .map_err(|_| "plugin host lock poisoned".to_string())?;
    open_plugin_webview(&app, &plugin_id, &project_id, &host, view_id.as_deref())
}

#[tauri::command]
pub(super) async fn plugin_host_view_data(
    state: tauri::State<'_, SharedCore>,
    plugins: tauri::State<'_, SharedPluginHost>,
    plugin_id: String,
    view_id: String,
    selected_entity_id: Option<String>,
) -> Result<serde_json::Value, String> {
    let project_id = current_info(state.inner())?
        .map(|info| info.root)
        .ok_or_else(|| "project is not open".to_string())?;
    let view = plugins
        .lock()
        .map_err(|_| "plugin host lock poisoned".to_string())?
        .host_view(&project_id, &plugin_id, &view_id)
        .map_err(|error| error.to_string())?;

    with_core(state, move |core| {
        let project = core.project(trusted_shell())?;
        let all_entities = project.list_entities()?;
        let mut lists = serde_json::Map::new();
        let mut list_entity_types = BTreeMap::new();
        for component in &view.components {
            if let ViewComponent::EntityList {
                id,
                entity_type,
                limit,
                ..
            } = component
            {
                let entities = all_entities
                    .iter()
                    .filter(|entity| {
                        !entity.deleted
                            && entity.entity_type.as_deref() == Some(entity_type.as_str())
                    })
                    .take(*limit as usize)
                    .cloned()
                    .collect::<Vec<_>>();
                lists.insert(
                    id.clone(),
                    serde_json::to_value(entities)
                        .map_err(|error| CoreError::Validation(error.to_string()))?,
                );
                list_entity_types.insert(id.clone(), entity_type.clone());
            }
        }

        let selected = selected_entity_id.as_deref().and_then(|id| {
            all_entities.iter().find(|entity| {
                !entity.deleted
                    && entity.id == id
                    && entity.entity_type.as_ref().is_some_and(|entity_type| {
                        list_entity_types
                            .values()
                            .any(|listed| listed == entity_type)
                    })
            })
        });
        let selected_id = selected.map(|entity| entity.id.as_str());
        let mut fields = serde_json::Map::new();
        if let Some(selected_id) = selected_id {
            for component in &view.components {
                if let ViewComponent::FieldForm {
                    source,
                    namespace,
                    fields: requested_fields,
                    ..
                } = component
                {
                    let Some(source_type) = list_entity_types.get(source) else {
                        continue;
                    };
                    let Some(selected_entity) = selected else {
                        continue;
                    };
                    if selected_entity.entity_type.as_deref() != Some(source_type.as_str()) {
                        continue;
                    }
                    for field in project.list_fields(selected_id.to_string())? {
                        if field.namespace == *namespace && requested_fields.contains(&field.key) {
                            fields.insert(field.key, field.value);
                        }
                    }
                }
            }
        }

        Ok(serde_json::json!({
            "lists": lists,
            "selected": selected,
            "fields": fields,
        }))
    })
    .await
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub(super) async fn plugin_host_view_set_field(
    state: tauri::State<'_, SharedCore>,
    plugins: tauri::State<'_, SharedPluginHost>,
    plugin_id: String,
    view_id: String,
    component_id: String,
    entity_id: String,
    key: String,
    value: serde_json::Value,
) -> Result<(), String> {
    let project_id = current_info(state.inner())?
        .map(|info| info.root)
        .ok_or_else(|| "project is not open".to_string())?;
    let view = plugins
        .lock()
        .map_err(|_| "plugin host lock poisoned".to_string())?
        .host_view(&project_id, &plugin_id, &view_id)
        .map_err(|error| error.to_string())?;
    let Some(ViewComponent::FieldForm {
        source,
        namespace,
        fields,
        editable: true,
        ..
    }) = view
        .components
        .iter()
        .find(|component| component_id == component_id_of(component))
    else {
        return Err("host field form is not declared or is read-only".into());
    };
    if !fields.iter().any(|field| field == &key) {
        return Err("field is not declared by the host field form".into());
    }
    let source_type = view
        .components
        .iter()
        .find_map(|component| match component {
            ViewComponent::EntityList {
                id, entity_type, ..
            } if id == source => Some(entity_type.as_str()),
            _ => None,
        })
        .ok_or_else(|| "host field form source is invalid".to_string())?
        .to_owned();
    let namespace = namespace.clone();
    with_core(state, move |core| {
        let project = core.project(trusted_shell())?;
        let entity = project
            .list_entities()?
            .into_iter()
            .find(|entity| entity.id == entity_id && !entity.deleted);
        if entity
            .as_ref()
            .and_then(|entity| entity.entity_type.as_deref())
            != Some(source_type.as_str())
        {
            return Err(CoreError::NotFound(
                "entity is outside the host view source".into(),
            ));
        }
        project.set_field(FieldValue {
            entity_id,
            namespace,
            key,
            value,
            revision: String::new(),
        })
    })
    .await
}

#[tauri::command]
pub(super) fn plugin_host_invoke_command(
    state: tauri::State<'_, SharedCore>,
    plugins: tauri::State<'_, SharedPluginHost>,
    plugin_id: String,
    view_id: Option<String>,
    command_id: String,
    payload: Option<serde_json::Value>,
) -> Result<String, String> {
    let project_id = current_info(state.inner())?
        .map(|info| info.root)
        .ok_or_else(|| "project is not open".to_string())?;
    let payload = payload.unwrap_or_else(|| serde_json::json!({}));
    let host = plugins
        .lock()
        .map_err(|_| "plugin host lock poisoned".to_string())?;
    let action = match view_id.as_deref() {
        Some(view_id) => {
            host.invoke_command_with_payload(&project_id, &plugin_id, view_id, &command_id, payload)
        }
        None => host.invoke_broker_command(&project_id, &plugin_id, &command_id, payload),
    }
    .map_err(|error| error.to_string())?;
    Ok(match action {
        CommandAction::RefreshView => "refresh-view".into(),
    })
}

pub(super) fn component_id_of(component: &ViewComponent) -> &str {
    match component {
        ViewComponent::Heading { id, .. }
        | ViewComponent::Text { id, .. }
        | ViewComponent::EntityList { id, .. }
        | ViewComponent::EntityDetail { id, .. }
        | ViewComponent::FieldForm { id, .. }
        | ViewComponent::Button { id, .. } => id,
    }
}

#[tauri::command]
pub(super) fn plugin_close_webview(app: tauri::AppHandle, plugin_id: String) -> Result<(), String> {
    close_plugin_webview(&app, &plugin_id);
    Ok(())
}

#[tauri::command]
pub(super) fn plugin_close_all_webviews(app: tauri::AppHandle) -> Result<(), String> {
    let labels: Vec<String> = app
        .webviews()
        .into_keys()
        .filter_map(|label| label.starts_with("plugin:").then_some(label))
        .collect();
    for label in labels {
        if let Ok(mut states) = embedded_webview_states().lock() {
            states.remove(&label);
        }
        if let Some(webview) = app.get_webview(&label) {
            let _ = webview.close();
        }
        if let Some(window) = app.get_webview_window(&label) {
            let _ = window.close();
        }
    }
    Ok(())
}
