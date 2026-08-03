<script lang="ts">
  import { onMount } from "svelte";
  import type { Entity, Relationship } from "$lib/project/client";

  let { pluginId, entities = [], relationships = [] }: { pluginId: string; entities?: Entity[]; relationships?: Relationship[] } = $props();

  let frame = $state<HTMLIFrameElement>();
  let ready = false;

  const isolatedDocument = `<!doctype html><meta charset="utf-8"><meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline'; script-src 'unsafe-inline'; connect-src 'none'; frame-src 'none'; object-src 'none'; img-src data:"><style>body{margin:0;padding:18px;font:14px system-ui;color:#d8d0c4;background:#211e1c}svg{width:100%;height:210px}.edge{stroke:#71675d;stroke-width:1}.node{fill:#c68d62}.label{text-anchor:middle;fill:#e9dfd2;font-size:12px}.type{text-anchor:middle;fill:#9f9488;font-size:10px}p{color:#9f9488}</style><div id="app"></div><script>window.addEventListener('message',function(event){if(!event.data||event.data.type!=='worldbuilder:projection')return;var app=document.getElementById('app');app.replaceChildren();var data=event.data;var title=document.createElement('strong');title.textContent=data.title;app.append(title);if(!data.entities.length){var empty=document.createElement('p');empty.textContent='Nothing to show yet.';app.append(empty);return;}var svg=document.createElementNS('http://www.w3.org/2000/svg','svg');svg.setAttribute('viewBox','0 0 720 230');var pos=new Map(data.entities.map(function(e,i){return[e.id,{x:70+(i%5)*145,y:70+Math.floor(i/5)*85}]}));data.relationships.forEach(function(r){var a=pos.get(r.source_id),b=pos.get(r.target_id);if(!a||!b)return;var l=document.createElementNS('http://www.w3.org/2000/svg','line');l.classList.add('edge');l.setAttribute('x1',a.x);l.setAttribute('y1',a.y);l.setAttribute('x2',b.x);l.setAttribute('y2',b.y);svg.append(l)});data.entities.forEach(function(e){var p=pos.get(e.id),c=document.createElementNS('http://www.w3.org/2000/svg','circle');c.classList.add('node');c.setAttribute('cx',p.x);c.setAttribute('cy',p.y);c.setAttribute('r',21);svg.append(c);var t=document.createElementNS('http://www.w3.org/2000/svg','text');t.classList.add('label');t.setAttribute('x',p.x);t.setAttribute('y',p.y+39);t.textContent=e.name.slice(0,20);svg.append(t);var k=document.createElementNS('http://www.w3.org/2000/svg','text');k.classList.add('type');k.setAttribute('x',p.x);k.setAttribute('y',p.y+53);k.textContent=e.entity_type||'entry';svg.append(k)});app.append(svg);parent.postMessage({type:'worldbuilder:projection-ready',pluginId:data.pluginId},'*')});parent.postMessage({type:'worldbuilder:projection-ready',pluginId:'INIT'},'*');<\/script>`;

  function sendProjection() {
    if (!ready || !frame?.contentWindow) return;
    frame.contentWindow.postMessage({ type: "worldbuilder:projection", pluginId, title: pluginId.endsWith("timeline") ? "Chronology" : "World graph", entities, relationships }, "*");
  }

  function receive(event: MessageEvent) {
    if (event.source !== frame?.contentWindow || event.data?.type !== "worldbuilder:projection-ready") return;
    if (event.data.pluginId === "INIT") { ready = true; sendProjection(); }
  }

  onMount(() => {
    window.addEventListener("message", receive);
    return () => window.removeEventListener("message", receive);
  });

  $effect(sendProjection);
</script>

<iframe bind:this={frame} title={`${pluginId} isolated view`} sandbox="allow-scripts" srcdoc={isolatedDocument}></iframe>

<style>
  iframe { width: 100%; min-height: 250px; border: 0; display: block; border-radius: 12px; background: #211e1c; }
</style>
