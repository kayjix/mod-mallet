// What a scan reports while it runs, made up for the browser preview: the
// steps of the real scan, with counts where the real one has them.

import type { ScanProgress } from "../generated/scan-progress";

type Step = ScanProgress["step"];

/** The steps in order, with how many items each has (0: not counted). */
const STEPS: [Step, number][] = [
  ["library", 0],
  ["archives", 209],
  ["meshes", 0],
  ["plugins", 398],
  ["mods", 0],
  ["models", 84],
  ["checking", 0],
  ["saving", 0],
];

const listeners = new Set<(progress: ScanProgress) => void>();

/** Calls `listener` with every report of a simulated scan. */
export function onSampleScanProgress(listener: (progress: ScanProgress) => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

/** Takes the steps in turn, reporting a few times a second like the engine. */
export async function simulateScan(): Promise<void> {
  const send = (step: Step, done: number, of: number) => {
    const stepNumber = STEPS.findIndex(([name]) => name === step) + 1;
    for (const listener of listeners) {
      listener({ step, stepNumber, steps: STEPS.length, done, of });
    }
  };
  for (const [step, of] of STEPS) {
    send(step, 0, 0);
    if (of === 0) {
      await sleep(step === "checking" ? 900 : 600);
      continue;
    }
    const slices = 12;
    for (let slice = 1; slice <= slices; slice += 1) {
      await sleep(200);
      send(step, Math.round((of * slice) / slices), of);
    }
  }
}
