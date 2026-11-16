import type { Model } from './models';
export function preferredModel(models: Model[]): string {
  return models.find((m) => /qwen.*3[._-]?5.*9b/i.test(m.id))?.id ?? models[0]?.id ?? '';
}
