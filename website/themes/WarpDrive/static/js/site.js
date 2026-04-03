(function () {
  'use strict';

  /* ---- Hyperdrive gauge: fills on scroll ---- */
  var gaugeFill = document.getElementById('gaugeFill');
  var chargePct = document.getElementById('chargePct');
  var prefersReducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;

  if (!prefersReducedMotion && gaugeFill && chargePct) {
    var ticking = false;
    window.addEventListener('scroll', function () {
      if (!ticking) {
        requestAnimationFrame(function () {
          var scrollTop = window.scrollY;
          var docHeight = document.documentElement.scrollHeight - window.innerHeight;
          var pct = Math.min(100, Math.round((scrollTop / docHeight) * 100));
          gaugeFill.style.width = pct + '%';
          chargePct.textContent = pct + '%';
          ticking = false;
        });
        ticking = true;
      }
    }, { passive: true });
  }

  /* ---- Scroll-triggered fade-in for sections ---- */
  var observer = new IntersectionObserver(function (entries) {
    entries.forEach(function (entry) {
      if (entry.isIntersecting) {
        entry.target.style.opacity = '1';
        entry.target.style.transform = 'translateY(0)';
        observer.unobserve(entry.target);
      }
    });
  }, { threshold: 0.1, rootMargin: '0px 0px -40px 0px' });

  document.querySelectorAll('.waypoint, .system-card, .indicator-panel, .role-card, .deploy-card, .post-card').forEach(function (el) {
    el.style.opacity = '0';
    el.style.transform = 'translateY(16px)';
    el.style.transition = 'opacity 0.5s cubic-bezier(0.23, 1, 0.32, 1), transform 0.5s cubic-bezier(0.23, 1, 0.32, 1)';
    observer.observe(el);
  });

  /* ---- Mobile nav burger toggle ---- */
  var burger = document.querySelector('.nav-burger');
  var mobileMenu = document.querySelector('.nav-mobile-menu');
  if (burger && mobileMenu) {
    burger.addEventListener('click', function () {
      var isOpen = mobileMenu.classList.toggle('is-open');
      burger.setAttribute('aria-expanded', isOpen ? 'true' : 'false');
    });
  }

  /* ---- Search modal ---- */
  var searchBtn = document.getElementById('nav-search');
  var searchModal = document.getElementById('search-modal');
  var searchClose = searchModal ? searchModal.querySelector('.modal-close') : null;
  var searchInput = searchModal ? searchModal.querySelector('#search') : null;

  if (searchBtn && searchModal) {
    searchBtn.addEventListener('click', function (e) {
      e.preventDefault();
      searchModal.classList.add('is-active');
      if (searchInput) searchInput.focus();
    });

    if (searchClose) {
      searchClose.addEventListener('click', function () {
        searchModal.classList.remove('is-active');
      });
    }

    searchModal.addEventListener('click', function (e) {
      if (e.target === searchModal || e.target.classList.contains('modal-background')) {
        searchModal.classList.remove('is-active');
      }
    });

    document.addEventListener('keydown', function (e) {
      if (e.key === 'Escape' && searchModal.classList.contains('is-active')) {
        searchModal.classList.remove('is-active');
      }
    });
  }

  /* ---- Elasticlunr search integration ---- */
  if (searchInput && typeof elasticlunr !== 'undefined') {
    var searchIndex = null;
    var resultsContainer = searchModal ? searchModal.querySelector('.search-results__items') : null;

    function loadIndex() {
      if (searchIndex) return;
      if (typeof window.searchIndex !== 'undefined') {
        searchIndex = elasticlunr.Index.load(window.searchIndex);
      }
    }

    function doSearch(query) {
      if (!searchIndex || !resultsContainer) return;
      resultsContainer.innerHTML = '';

      if (query.length < 2) return;

      var results = searchIndex.search(query, {
        fields: { title: { boost: 2 }, body: { boost: 1 } },
        bool: 'OR',
        expand: true
      });

      if (results.length === 0) {
        resultsContainer.innerHTML = '<p class="search-no-results">No results found.</p>';
        return;
      }

      results.slice(0, 10).forEach(function (result) {
        var item = searchIndex.documentStore.getDoc(result.ref);
        if (!item) return;

        var div = document.createElement('div');
        div.className = 'search-result';
        div.innerHTML = '<a href="' + item.url + '" class="search-result-link">' +
          '<h4 class="search-result-title">' + (item.title || 'Untitled') + '</h4>' +
          '<p class="search-result-body">' + (item.body || '').substring(0, 150) + '...</p>' +
          '</a>';
        resultsContainer.appendChild(div);
      });
    }

    searchInput.addEventListener('input', function () {
      loadIndex();
      doSearch(this.value);
    });

    searchInput.addEventListener('focus', function () {
      loadIndex();
    });
  }

  /* ---- Copy button ---- */
  document.querySelectorAll('.copy-btn').forEach(function (btn) {
    btn.addEventListener('click', function () {
      var text = btn.getAttribute('data-copy') || btn.previousElementSibling.textContent;
      navigator.clipboard.writeText(text.trim()).then(function () {
        var orig = btn.textContent;
        btn.textContent = 'copied';
        setTimeout(function () { btn.textContent = orig; }, 1500);
      });
    });
  });
})();
