// ABOUTME: Tests for the header bar's presentational rendering and for the
// ABOUTME: App-level wiring that feeds it a validation error count from the engine.

import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { cleanup, render, screen } from '@testing-library/react';
import type { ReactNode } from 'react';
import { HeaderBar } from '../../src/components/HeaderBar';
import { AppContent } from '../../src/App';
import { EngineProvider } from '../../src/engine/useEngine';
import type { Engine } from '../../src/engine/engine';
import { initialState, useForgeStore } from '../../src/model/store';
import type { ManifestState } from '../../src/model/types';

afterEach(cleanup);

const manifest: ManifestState = {
  name: 'Border Marches',
  version: '1.0',
  namespace: 'border-marches',
  author: '',
  minToolVersion: '',
};

describe('HeaderBar', () => {
  it('renders the brand block and the collection name', () => {
    render(<HeaderBar manifest={manifest} errorCount={0} />);

    expect(screen.getByText('Fatescroll')).toBeTruthy();
    expect(screen.getByText('TABLE FORGE')).toBeTruthy();
    expect(screen.getByText('Collection')).toBeTruthy();
    expect(screen.getByText('Border Marches')).toBeTruthy();
  });

  it('shows a valid status pill with zero errors', () => {
    render(<HeaderBar manifest={manifest} errorCount={0} />);
    const text = screen.getByText('Collection is valid');
    expect(text).toBeTruthy();
    expect(text.closest('.header-status')?.className).toContain('header-status--valid');
  });

  it('shows an invalid status pill with a pluralized error count', () => {
    render(<HeaderBar manifest={manifest} errorCount={3} />);
    const text = screen.getByText('3 errors');
    expect(text).toBeTruthy();
    expect(text.closest('.header-status')?.className).toContain('header-status--invalid');
  });

  it('singularizes a single error', () => {
    render(<HeaderBar manifest={manifest} errorCount={1} />);
    expect(screen.getByText('1 error')).toBeTruthy();
  });

  it('renders a disabled export button', () => {
    render(<HeaderBar manifest={manifest} errorCount={0} />);
    const button = screen.getByRole('button', { name: /export collection/i }) as HTMLButtonElement;
    expect(button.disabled).toBe(true);
  });
});

function makeFakeEngine(errors: string[]): Engine {
  return {
    validate: () => errors,
    diceInfo: () => ({ ok: false }),
    expectedValues: () => null,
    histogram: () => null,
    roll: () => ({ error: 'not used' }),
  };
}

function wrapper(engine: Engine) {
  return ({ children }: { children: ReactNode }) => (
    <EngineProvider engine={engine} debounceMs={0}>
      {children}
    </EngineProvider>
  );
}

describe('AppContent wiring', () => {
  beforeEach(() => {
    useForgeStore.setState(initialState());
  });

  it('passes zero engine validation errors through to a valid status pill', () => {
    const Wrapper = wrapper(makeFakeEngine([]));
    render(
      <Wrapper>
        <AppContent />
      </Wrapper>,
    );
    expect(screen.getByText('Collection is valid')).toBeTruthy();
  });

  it('passes engine validation errors through to the error-count status pill', () => {
    const Wrapper = wrapper(makeFakeEngine(['bad namespace', 'bad dice']));
    render(
      <Wrapper>
        <AppContent />
      </Wrapper>,
    );
    expect(screen.getByText('2 errors')).toBeTruthy();
  });
});
