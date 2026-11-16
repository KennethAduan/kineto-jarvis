import { useReducedMotion } from 'motion/react';
import { ThinkingOrb, type OrbSize, type OrbState } from 'thinking-orbs';

/** Decorative status feedback; the adjacent text carries the accessible status. */
export function ActivityOrb({ state, size = 20 }: { state: OrbState; size?: OrbSize }) {
  const reducedMotion = useReducedMotion();
  return (
    <ThinkingOrb
      state={state}
      size={size}
      theme="dark"
      color="#a8e7d1"
      paused={reducedMotion === true}
      aria-hidden="true"
      data-orb-state={state}
      data-orb-paused={reducedMotion === true}
    />
  );
}
