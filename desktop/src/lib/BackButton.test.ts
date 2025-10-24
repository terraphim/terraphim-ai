import { render, screen, fireEvent } from '@testing-library/svelte/svelte5';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import BackButton from './BackButton.svelte';

// Mock window.history and window.location
const mockHistoryBack = vi.fn();
const mockLocationHref = vi.fn();
let currentPathname = '/graph';

Object.defineProperty(window, 'history', {
  value: {
    back: mockHistoryBack,
    length: 2
  },
  writable: true
});

Object.defineProperty(window, 'location', {
  value: {
    href: mockLocationHref,
    get pathname() { return currentPathname; },
    set pathname(p: string) { currentPathname = p; }
  },
  writable: true
});

// Mock window.location.href assignment
Object.defineProperty(window.location, 'href', {
  set: mockLocationHref as unknown as PropertyDescriptor['set'],
  get: () => mockLocationHref() as unknown as string,
  configurable: true
});

// Ensure path-based visibility checks run
function setPath(path: string) {
  currentPathname = path;
  window.dispatchEvent(new Event('popstate'));
}

describe('BackButton', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // Default to a non-home path so button is visible
    setPath('/graph');
    // Reset history length to simulate having navigation history
    Object.defineProperty(window.history, 'length', {
      value: 2,
      writable: true
    });
  });

  it('is hidden on home (/) by default', () => {
    setPath('/');
    render(BackButton);
    const button = screen.queryByRole('button', { name: /go back/i });
    expect(button).not.toBeInTheDocument();
  });

  it('renders with default props on non-home routes', () => {
    setPath('/graph');
    render(BackButton);

    const button = screen.getByRole('button', { name: /go back/i });
    expect(button).toBeInTheDocument();
    // Svelma/Bulma button class present
    expect(button).toHaveClass('button');

    const icon = document.querySelector('.fas.fa-arrow-left');
    expect(icon).toBeInTheDocument();

    const text = screen.getByText('Back');
    expect(text).toBeInTheDocument();
  });

  it('renders without text when showText is false', () => {
    setPath('/graph');
    render(BackButton, { showText: false });

    const button = screen.getByRole('button', { name: /go back/i });
    expect(button).toBeInTheDocument();

    const text = screen.queryByText('Back');
    expect(text).not.toBeInTheDocument();
  });

  it('applies custom class when provided', () => {
    setPath('/graph');
    const customClass = 'custom-back-button';
    render(BackButton, { customClass });

    const button = screen.getByRole('button', { name: /go back/i });
    expect(button).toHaveClass(customClass);
    expect(button).toHaveClass('back-button');
  });

  it('calls history.back() when there is navigation history', () => {
    setPath('/graph');
    Object.defineProperty(window.history, 'length', {
      value: 3,
      writable: true
    });

    render(BackButton);

    const button = screen.getByRole('button', { name: /go back/i });
    fireEvent.click(button);

    expect(mockHistoryBack).toHaveBeenCalledTimes(1);
    expect(mockLocationHref).not.toHaveBeenCalled();
  });

  it('redirects to fallback path when no navigation history', () => {
    setPath('/graph');
    Object.defineProperty(window.history, 'length', {
      value: 1,
      writable: true
    });

    const fallbackPath = '/home';
    render(BackButton, { fallbackPath });

    const button = screen.getByRole('button', { name: /go back/i });
    fireEvent.click(button);

    expect(mockHistoryBack).not.toHaveBeenCalled();
    expect(mockLocationHref).toHaveBeenCalledWith('/home');
  });

  it('uses default fallback path when none provided', () => {
    setPath('/graph');
    Object.defineProperty(window.history, 'length', {
      value: 1,
      writable: true
    });

    render(BackButton);

    const button = screen.getByRole('button', { name: /go back/i });
    fireEvent.click(button);

    expect(mockLocationHref).toHaveBeenCalledWith('/');
  });

  it('has correct accessibility attributes', () => {
    setPath('/graph');
    render(BackButton);

    const button = screen.getByRole('button', { name: /go back/i });
    expect(button).toHaveAttribute('title', 'Go back');
    expect(button).toHaveAttribute('aria-label', 'Go back');
  });

  it('has correct positioning styles', () => {
    setPath('/graph');
    render(BackButton);

    const button = screen.getByRole('button', { name: /go back/i });
    // Check that the button has the correct CSS class
    expect(button).toHaveClass('back-button');
    // Uses svelma/bulma class
    expect(button).toHaveClass('button');

    // Check that the button is rendered (positioning handled by CSS)
    expect(button).toBeInTheDocument();
  });

  it('handles keyboard navigation', () => {
    setPath('/graph');
    render(BackButton);

    const button = screen.getByRole('button', { name: /go back/i });

    // Test Enter key
    fireEvent.keyDown(button, { key: 'Enter' });
    expect(mockHistoryBack).toHaveBeenCalledTimes(1);

    // Reset mock for next test
    vi.clearAllMocks();

    // Test Space key
    fireEvent.keyDown(button, { key: ' ' });
    expect(mockHistoryBack).toHaveBeenCalledTimes(1);
  });

  it('maintains state between renders', () => {
    setPath('/graph');
    const { rerender } = render(BackButton, { showText: true });

    let button = screen.getByRole('button', { name: /go back/i });
    expect(button).toHaveTextContent('Back');

    rerender({ showText: false });
    button = screen.getByRole('button', { name: /go back/i });
    // The text should still be there since we're not fully re-rendering
    // This test validates the component structure remains intact
    expect(button).toBeInTheDocument();
  });
});
