import { mount, unmount } from "svelte";
import type { DaenaModule, ModuleContext, ModuleManifest } from "../../../module-api/src/index";
import manifestJson from "../manifest.json";
import LanguageWorkspace from "./LanguageWorkspace.svelte";

const manifest = manifestJson as unknown as ModuleManifest;

export const language: DaenaModule = {
  manifest,
  views: [
    {
      id: "lexicon",
      title: "Lexicon",
      mount(element: HTMLElement, context: ModuleContext) {
        const app = mount(LanguageWorkspace, { target: element, props: { context } });
        return () => unmount(app);
      },
    },
  ],
};
