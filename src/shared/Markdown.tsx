import { useMemo } from 'react';
import DOMPurify from 'dompurify';
import { marked } from 'marked';
import { call } from './bridge';
import { run } from './components/Controls';

marked.setOptions({ gfm: true, breaks: false });

export function Markdown({ content }: { content: string }) {
  const html = useMemo(() => {
    const parsed = marked.parse(content, { async: false });
    return DOMPurify.sanitize(typeof parsed === 'string' ? parsed : '');
  }, [content]);
  return <div className="markdown-body" onClick={e => {
    const link = (e.target as HTMLElement).closest('a');
    if (!link) return;
    e.preventDefault();
    const href = link.getAttribute('href');
    if (href && /^https?:\/\//i.test(href)) void run(() => call('open_external', { url: href }));
  }} dangerouslySetInnerHTML={{ __html: html }}/>;
}
