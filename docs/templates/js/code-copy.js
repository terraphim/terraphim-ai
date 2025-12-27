document.addEventListener('DOMContentLoaded', () => {
  document.querySelectorAll('pre code').forEach(codeBlock => {
    const copyButton = document.createElement('sl-copy-button');
    copyButton.value = codeBlock.textContent;
    copyButton.size = 'small';
    copyButton.variant = 'neutral';
    codeBlock.parentElement.appendChild(copyButton);
  });
});