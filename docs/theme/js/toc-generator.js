document.addEventListener('DOMContentLoaded', () => {
  const content = document.querySelector('main');
  const tocContent = document.getElementById('toc-content');
  
  // Early return if required elements don't exist
  if (!content || !tocContent) {
      console.debug('TOC generation skipped: required elements not found');
      return;
  }

  // Generate TOC from content headings
  const headings = content.querySelectorAll('h1, h2, h3, h4');
  if (headings.length === 0) {
      console.debug('TOC generation skipped: no headings found');
      return;
  }

  const toc = document.createElement('ul');
  toc.className = 'toc-list';

  const tocItems = new Map();
  let currentLevel = 1;
  let currentParent = toc;
  let previousElement = null;

  headings.forEach((heading) => {
      // Get heading level number
      const level = parseInt(heading.tagName.charAt(1));
      
      // Create TOC item
      const item = document.createElement('li');
      item.setAttribute('data-level', level.toString());
      item.classList.add(`toc-level-${level}`);
      
      const link = document.createElement('a');
      
      // Generate or get id for heading
      if (!heading.id) {
          heading.id = heading.textContent
              .toLowerCase()
              .replace(/[^a-z0-9]+/g, '-')
              .replace(/(^-|-$)/g, '');
      }
      
      link.href = `#${heading.id}`;
      link.textContent = heading.textContent;
      item.appendChild(link);

      // Handle nesting based on heading level
      if (level > currentLevel) {
          const newList = document.createElement('ul');
          newList.classList.add('toc-sublist');
          previousElement.appendChild(newList);
          currentParent = newList;
      } else if (level < currentLevel) {
          const levelDiff = currentLevel - level;
          for (let i = 0; i < levelDiff; i++) {
              currentParent = currentParent.parentElement?.parentElement || toc;
          }
      } else if (level === currentLevel && currentParent !== toc) {
          // Stay at same level but ensure we're in the correct parent
          currentParent = currentParent.parentElement?.parentElement || toc;
      }

      currentParent.appendChild(item);
      previousElement = item;
      currentLevel = level;
      
      // Store reference for scroll highlighting
      tocItems.set(heading, link);
      
      // Add click handler
      link.addEventListener('click', (e) => {
          e.preventDefault();
          heading.scrollIntoView({ behavior: 'smooth' });
          history.pushState(null, '', link.href);
          
          // Update active state
          tocContent.querySelectorAll('.active').forEach(el => el.classList.remove('active'));
          link.classList.add('active');
      });
  });

  tocContent.appendChild(toc);

  // Add intersection observer for active section highlighting
  const observerOptions = {
      root: null,
      rootMargin: '-80px 0px -80% 0px',
      threshold: 1.0
  };

  const observer = new IntersectionObserver((entries) => {
      entries.forEach(entry => {
          const link = tocItems.get(entry.target);
          if (!link) return;

          if (entry.isIntersecting) {
              // Remove previous active class
              tocContent.querySelectorAll('.active').forEach(el => el.classList.remove('active'));
              // Add active class to current section
              link.classList.add('active');
          }
      });
  }, observerOptions);

  headings.forEach(heading => observer.observe(heading));
});