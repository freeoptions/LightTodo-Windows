import { onMounted, onUnmounted, ref } from 'vue';

function localDay(date: Date): string {
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}-${String(date.getDate()).padStart(2, '0')}`;
}

/** Keep the local calendar day current, including after suspended WebView timers. */
export function useLocalDay(refresh: (previousDay: string) => Promise<boolean>) {
  const today = ref(localDay(new Date()));
  let refreshedDay = today.value;
  let refreshing = false;
  let stopped = false;
  let midnightTimer: ReturnType<typeof setTimeout> | undefined;

  const checkDay = async () => {
    today.value = localDay(new Date());
    if (stopped || refreshing || refreshedDay === today.value) return;

    refreshing = true;
    const targetDay = today.value;
    try {
      if (await refresh(refreshedDay)) refreshedDay = targetDay;
    } catch (error) {
      console.error('Failed to refresh todos after date change:', error);
    } finally {
      refreshing = false;
    }
  };

  const scheduleMidnight = () => {
    if (stopped) return;
    clearTimeout(midnightTimer);
    const now = new Date();
    const midnight = new Date(now.getFullYear(), now.getMonth(), now.getDate() + 1);
    midnightTimer = setTimeout(() => {
      void checkDay();
      scheduleMidnight();
    }, midnight.getTime() - now.getTime());
  };

  const handleResume = () => {
    void checkDay();
    // Recalculate after sleep or a system clock/timezone change.
    scheduleMidnight();
  };

  const handleVisibility = () => {
    if (document.visibilityState === 'visible') handleResume();
  };

  onMounted(() => {
    void checkDay();
    scheduleMidnight();
    window.addEventListener('focus', handleResume);
    document.addEventListener('visibilitychange', handleVisibility);
  });

  onUnmounted(() => {
    stopped = true;
    clearTimeout(midnightTimer);
    window.removeEventListener('focus', handleResume);
    document.removeEventListener('visibilitychange', handleVisibility);
  });

  return today;
}
