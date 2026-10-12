/* Attach one export format to the existing acknowledged-command barrier. */
(function (root) {
  'use strict';
  class TerraphimExportIntent {
    begin(format = 'markdown') {
      if (!['markdown', 'knowledge-graph'].includes(format)) throw new Error('Unsupported export format');
      if (this.pending) return false;
      this.pending = { format, id: null };
      return true;
    }
    attach(message) {
      if (message.type !== 'export') return message;
      if (!this.pending || this.pending.id !== null) throw new Error('Export intent is unavailable');
      this.pending.id = message.id;
      return { ...message, format: this.pending.format };
    }
    settle(message, accepted) {
      if (!accepted || !this.pending) return;
      if (message.type === 'reject' ||
          (message.type === 'command-complete' && message.id === this.pending.id)) this.clear();
    }
    clear() { this.pending = null; }
  }
  if (typeof module === 'object' && module.exports) module.exports = { TerraphimExportIntent };
  else root.TerraphimExportIntent = TerraphimExportIntent;
})(typeof window === 'object' ? window : globalThis);
