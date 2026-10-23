import { describe, expect, test } from 'bun:test';
import { renderToStaticMarkup } from 'react-dom/server';
import { EmptyStateSummary } from '../../src/components/EmptyStateSummary';

describe('empty summary state', () => {
  test('renders a persistent error with a way to fix the model', () => {
    const html = renderToStaticMarkup(
      <EmptyStateSummary
        error="Summary request failed"
        hasModel
        onChooseModel={() => {}}
      />,
    );

    expect(html).toContain('role="alert"');
    expect(html).toContain('Summary request failed');
    expect(html).toContain('<button');
  });
});
