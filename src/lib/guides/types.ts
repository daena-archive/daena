export interface GuideStep {
  id: string;
  title: string;
  body: string;
  target?: string;
  waitForTarget?: boolean;
  primaryLabel?: string;
  action?: string;
}

export type GuideMode = "tour" | "hint";
