import type { Model } from './models';
export const DEFAULT_MODEL = 'qwen/qwen3-4b-2507';
export function preferredModel(models: Model[]): string {
  return (
    models.find((m) => m.id === DEFAULT_MODEL)?.id ??
    models.find((m) => /qwen3[-_]4b[-_]2507/i.test(m.id))?.id ??
    models[0]?.id ??
    ''
  );
}
