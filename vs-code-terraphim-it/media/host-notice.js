/* Recoverable host notices must never become an undismissable editor overlay. */
(function (root) {
  'use strict';
  class TerraphimHostNotice {
    constructor(element) { this.element = element; }
    recoverable(message) {
      const document = this.element.ownerDocument;
      const text = document.createElement('span'); text.textContent = message;
      const dismiss = document.createElement('button');
      dismiss.type = 'button'; dismiss.textContent = 'Dismiss';
      dismiss.dataset.dismissNotice = 'true';
      dismiss.setAttribute('aria-label', 'Dismiss Terraphim notice');
      dismiss.addEventListener('click', () => {
        if (this.element.dataset.kind !== 'notice') return;
        this.element.hidden = true;
        this.element.replaceChildren();
      });
      this.element.dataset.kind = 'notice';
      this.element.replaceChildren(text, dismiss);
      this.element.hidden = false;
    }
    blocking() {
      this.element.dataset.kind = 'blocking';
      this.element.querySelector('[data-dismiss-notice]')?.remove();
    }
  }
  root.TerraphimHostNotice = TerraphimHostNotice;
})(globalThis);
