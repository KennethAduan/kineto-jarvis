import { useEffect, useState } from 'react';
import { ActivityOrb } from '@/components/feedback/ActivityOrb';
import * as motion from 'motion/react-m';
export function GenerationStatus({ thinking }: { thinking: boolean }) {
  const [started] = useState(Date.now);
  const [seconds, setSeconds] = useState(0);
  useEffect(() => {
    const timer = window.setInterval(
      () => setSeconds(Math.floor((Date.now() - started) / 1000)),
      1000,
    );
    return () => window.clearInterval(timer);
  }, [started]);
  return (
    <motion.div
      initial={{ opacity: 0, y: 4 }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0 }}
      className="generation-status"
      aria-live="off"
    >
      <ActivityOrb state={thinking ? 'solving' : 'composing'} />
      <span>
        {thinking ? 'JARVIS is thinking' : 'Generating locally'}
        <small>Your local model is working. You can stop at any time.</small>
      </span>
      <time>
        {Math.floor(seconds / 60)}:{String(seconds % 60).padStart(2, '0')}
      </time>
    </motion.div>
  );
}
