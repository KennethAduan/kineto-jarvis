import { memo, useDeferredValue } from 'react';
import Markdown, { type Components } from 'react-markdown';
import remarkGfm from 'remark-gfm';

const plugins = [remarkGfm];
const elements = [
  'p',
  'strong',
  'em',
  'del',
  'ul',
  'ol',
  'li',
  'blockquote',
  'pre',
  'code',
  'h1',
  'h2',
  'h3',
  'h4',
  'h5',
  'h6',
  'hr',
  'table',
  'thead',
  'tbody',
  'tr',
  'th',
  'td',
  'br',
  'a',
  'img',
  'input',
  'section',
  'sup',
];
const components: Components = {
  h1: ({ children }) => <h2>{children}</h2>,
  h2: ({ children }) => <h3>{children}</h3>,
  h3: ({ children }) => <h4>{children}</h4>,
  // Model output cannot navigate the WebView or load remote images.
  a: ({ children, href }) => (
    <span className="markdown-link" title={href}>
      {children}
      {href && children !== href && <span className="markdown-url"> ({href})</span>}
    </span>
  ),
  img: ({ alt }) => <span className="markdown-image">[Image{alt ? `: ${alt}` : ''}]</span>,
  input: ({ checked }) => (
    <input
      type="checkbox"
      checked={checked ?? false}
      disabled
      aria-label={checked ? 'Completed task' : 'Incomplete task'}
    />
  ),
  table: ({ children }) => (
    <div className="markdown-table" role="region" aria-label="Response table" tabIndex={0}>
      <table>{children}</table>
    </div>
  ),
};

/** Render only assistant output. Raw HTML, navigation and external media stay disabled. */
export default memo(function MessageContent({ content }: { content: string }) {
  const deferredContent = useDeferredValue(content);
  return (
    <div className="markdown">
      <Markdown skipHtml allowedElements={elements} remarkPlugins={plugins} components={components}>
        {deferredContent}
      </Markdown>
    </div>
  );
});
