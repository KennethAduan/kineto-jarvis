import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import MessageContent from './MessageContent';
afterEach(cleanup);

describe('assistant Markdown', () => {
  it('formats headings, lists, code and tables', () => {
    const { container } = render(
      <MessageContent
        content={
          '# Plan\n\n- **Focus** first\n- Take a break\n\n```ts\nconst value = 1;\n```\n\n| Task | Time |\n| --- | --- |\n| Build | Morning |'
        }
      />,
    );
    expect(screen.getByRole('heading', { name: 'Plan', level: 2 })).toBeDefined();
    expect(container.querySelector('strong')?.textContent).toBe('Focus');
    expect(container.querySelectorAll('li')).toHaveLength(2);
    expect(container.querySelector('pre code')?.textContent).toContain('const value = 1;');
    expect(screen.getByRole('table')).toBeDefined();
  });
  it('never executes HTML, navigates links, or loads remote images', () => {
    const { container } = render(
      <MessageContent
        content={
          '<script>alert(1)</script>\n\n<img src="https://example.com/tracker" onerror="alert(2)">\n\n[Unsafe](javascript:alert(3))\n\n[Docs](https://example.com/docs)\n\n![Remote diagram](https://example.com/diagram.png)'
        }
      />,
    );
    expect(container.querySelector('script,img,iframe,a')).toBeNull();
    expect(screen.getByText('[Image: Remote diagram]')).toBeDefined();
    expect(screen.getByText('(https://example.com/docs)')).toBeDefined();
  });
  it('renders an unfinished streaming code block as text', () => {
    const { container } = render(
      <MessageContent content={'```html\n<img src=x onerror=alert(1)>'} />,
    );
    expect(container.querySelector('pre code')?.textContent).toContain('<img');
    expect(container.querySelector('img')).toBeNull();
  });
});
