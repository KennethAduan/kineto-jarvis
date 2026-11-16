import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import { LazyMotion, domAnimation, MotionConfig } from 'motion/react';
import './ui.css';
import './style.css';

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <LazyMotion features={domAnimation} strict>
      <MotionConfig reducedMotion="user" transition={{ duration: 0.2 }}>
        <App />
      </MotionConfig>
    </LazyMotion>
  </React.StrictMode>,
);
