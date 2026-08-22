import Image from "@tiptap/extension-image";
import { resolveAssetSrc, retainAssetUrl, releaseAssetUrl } from "$lib/assets/resolve";

const PLACEHOLDER = "data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7";

export const AssetImage = Image.extend({
  addNodeView() {
    return ({ node, getPos, editor }) => {
      const wrapper = document.createElement("span");
      wrapper.className = "daena-asset-image-wrapper";
      wrapper.style.display = "inline-block";
      wrapper.style.lineHeight = "0";

      const img = document.createElement("img");
      img.alt = node.attrs.alt ?? "";
      if (node.attrs.title) img.title = node.attrs.title;
      img.className = node.attrs.HTMLAttributes?.class ?? "daena-content-image";
      img.style.maxWidth = "100%";
      img.style.height = "auto";
      img.style.borderRadius = "6px";
      img.style.border = "1px solid var(--line, #e4e1d8)";

      const applyDims = (attrs: Record<string, unknown>) => {
        const wRaw = attrs.width;
        const hRaw = attrs.height;
        const w = wRaw != null && /^\d+$/.test(String(wRaw).trim()) ? String(wRaw).trim() : "";
        const h = hRaw != null && /^\d+$/.test(String(hRaw).trim()) ? String(hRaw).trim() : "";
        if (w) {
          img.setAttribute("width", w);
          img.style.width = `${w}px`;
          try {
            img.width = Number(w);
          } catch {}
        } else {
          img.removeAttribute("width");
          img.style.width = "";
        }
        if (h) {
          img.setAttribute("height", h);
          img.style.height = `${h}px`;
          try {
            img.height = Number(h);
          } catch {}
        } else {
          img.removeAttribute("height");
          img.style.height = "auto";
        }
      };
      applyDims(node.attrs as Record<string, unknown>);

      const originalSrc: string = node.attrs.src ?? "";
      let currentAttrs: Record<string, unknown> = { ...(node.attrs as Record<string, unknown>) };
      let currentBlob: string | null = null;
      let disposed = false;

      const setBlob = (blobUrl: string) => {
        if (disposed) return;
        if (currentBlob) releaseAssetUrl(currentBlob);
        currentBlob = blobUrl;
        retainAssetUrl(blobUrl);
        img.src = blobUrl;
      };

      const handleError = () => {
        if (!originalSrc.startsWith("assets/")) return;
        if (!img.src.startsWith("blob:")) return;
        // Blob was revoked or failed — attempt re-resolve once
        void resolveAssetSrc(originalSrc).then((blobUrl) => {
          if (!blobUrl || disposed) return;
          if (img.isConnected && img.dataset.assetSrc === originalSrc && img.src !== blobUrl) {
            setBlob(blobUrl);
          }
        });
      };
      img.addEventListener("error", handleError);

      // For external or already blob, use directly
      if (!originalSrc.startsWith("assets/")) {
        img.src = originalSrc;
        if (!originalSrc) img.style.display = "none";
      } else {
        // Use placeholder initially to avoid 404 fetch to /assets/...
        img.dataset.originalSrc = originalSrc;
        img.src = PLACEHOLDER;
        img.dataset.assetSrc = originalSrc;
        // Async resolve to blob
        void resolveAssetSrc(originalSrc).then((blobUrl) => {
          if (!blobUrl) {
            // keep placeholder but show error styling
            img.style.opacity = "0.85";
            img.style.border = "1px dashed #e8c0b8";
            img.title = `Image unavailable: ${originalSrc}`;
            // keep alt visible via title
            return;
          }
          // Only update if still attached and src still placeholder (avoid overwriting if node updated)
          if (!disposed && img.isConnected && img.dataset.assetSrc === originalSrc) {
            setBlob(blobUrl);
          }
        });
      }

      wrapper.appendChild(img);

      return {
        dom: wrapper,
        contentDOM: null,
        update: (updatedNode) => {
          if (updatedNode.type.name !== "image") return false;
          // Update alt/title if changed
          if (updatedNode.attrs.alt !== currentAttrs.alt) img.alt = updatedNode.attrs.alt ?? "";
          if (updatedNode.attrs.title !== currentAttrs.title) img.title = updatedNode.attrs.title ?? "";
          const newW = updatedNode.attrs.width;
          const newH = updatedNode.attrs.height;
          const oldW = currentAttrs.width;
          const oldH = currentAttrs.height;
          if (String(newW ?? "") !== String(oldW ?? "") || String(newH ?? "") !== String(oldH ?? "")) {
            applyDims(updatedNode.attrs as Record<string, unknown>);
          }
          const newSrc: string = updatedNode.attrs.src ?? "";
          if (newSrc !== originalSrc) {
            // src changed — re-resolve if asset path
            // For simplicity, recreate: easiest is to signal false to recreate nodeView
            return false;
          }
          currentAttrs = { ...(updatedNode.attrs as Record<string, unknown>) };
          return true;
        },
        destroy() {
          disposed = true;
          img.removeEventListener("error", handleError);
          if (currentBlob) {
            releaseAssetUrl(currentBlob);
            currentBlob = null;
          }
        },
        // Ensure getPos still works for ProseMirror mapping
        selectNode: () => {
          img.classList.add("ProseMirror-selectednode");
        },
        deselectNode: () => {
          img.classList.remove("ProseMirror-selectednode");
        },
      };
    };
  },
});
