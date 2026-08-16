(function (root, factory) {
  const api = factory();
  if (typeof module === "object" && module.exports) module.exports = api;
  else root.LatestRequest = api;
}(typeof globalThis !== "undefined" ? globalThis : this, () => {
  function create({ execute, apply, onError = () => {} }) {
    let running = false;
    let pending = null;
    let latestSequence = 0;

    async function drain() {
      if (running) return;
      running = true;
      while (pending) {
        const job = pending;
        pending = null;
        try {
          const result = await execute(job.value);
          const current = job.sequence === latestSequence;
          if (current) apply(result);
          job.resolve({ applied: current, result });
        } catch (error) {
          const current = job.sequence === latestSequence;
          if (current) onError(error);
          job.resolve({ applied: false, error });
        }
      }
      running = false;
    }

    function submit(value) {
      const sequence = ++latestSequence;
      return new Promise((resolve) => {
        if (pending) pending.resolve({ applied: false, skipped: true });
        pending = { sequence, value, resolve };
        void drain();
      });
    }

    function invalidate() {
      latestSequence += 1;
      if (pending) pending.resolve({ applied: false, skipped: true });
      pending = null;
    }

    return { submit, invalidate };
  }

  return { create };
}));
