import { type ThemeTokenId } from "./generated.js";
export type ThemeMode = "light" | "dark";
export type ThemeTokenMap = Record<ThemeTokenId, string>;
export declare function parseThemeColor(value: string): string;
export declare function mergeThemeTokens(base: ThemeTokenMap, overlay: Record<string, string>): ThemeTokenMap;
export declare function resolveThemeTokens(mode: ThemeMode, overlay: Record<string, string>): ThemeTokenMap;
export declare function validateThemeContrast(tokens: ThemeTokenMap): void;
export declare function validateThemePack(pack: {
    id: string;
    name: string;
    tokens: {
        light: unknown;
        dark: unknown;
    };
}): string[];
//# sourceMappingURL=theme.d.ts.map