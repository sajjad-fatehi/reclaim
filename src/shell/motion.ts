import { useEffect, useRef } from 'react';
import type { RefObject } from 'react';

export function usePanelMotion(
  reference: RefObject<HTMLElement | null>,
  visible: boolean,
  direction: 'view' | 'edge' = 'view',
): void {
  const previousVisibility = useRef(visible);
  useEffect(() => {
    const entering = visible && !previousVisibility.current;
    previousVisibility.current = visible;
    if (!entering) return;
    const element = reference.current;
    if (!element || !visible || !element.animate) return;
    const preference = window.matchMedia('(prefers-reduced-motion: reduce)');
    if (preference.matches) return;
    const tokens = getComputedStyle(element);
    const duration =
      Number.parseFloat(
        tokens.getPropertyValue(
          direction === 'edge' ? '--duration-panel' : '--duration-selection',
        ),
      ) || 0;
    const animation = element.animate(
      [
        {
          opacity: 0.6,
          transform:
            direction === 'edge' ? 'translateX(20px)' : 'translateY(6px)',
        },
        { opacity: 1, transform: 'translate(0)' },
      ],
      {
        duration,
        easing: tokens.getPropertyValue('--ease-out').trim() || 'ease-out',
      },
    );
    function stopMotion(): void {
      animation.cancel();
    }
    preference.addEventListener('change', stopMotion);
    return () => {
      animation.cancel();
      preference.removeEventListener('change', stopMotion);
    };
  }, [reference, visible, direction]);
}
