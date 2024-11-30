(() => {
  if (localStorage.isDark === "true") {
    $("html").classList.add("dark");
  } else {
    $("html").classList.remove("dark");
  }

  lm.config({
    alias: {
      "@obook": "https://cdn.jsdelivr.net/npm/obook@2.1.41",
      // "@obook": "http://127.0.0.1:5512",
    },
  });

  const url = import.meta.url;
  function resolvePath(path) {
    return new URL(path, url).href;
  }

  const load = lm(import.meta);

  Promise.all([
    new Promise((res) => {
      let timer;
      $("body").one("page-ready", () => {
        res();
        clearTimeout(timer);
      });

      timer = setTimeout(res, 5000);
    }),
    load("./comps/article-nav/article-nav.html"),
    load("./comps/doc-code.html"),
    load("./comps/doc-header/doc-header.html"),
    load("./comps/doc-search/doc-search.html"),
    load("./comps/article-container.html"),
    load("./comps/article-footer.html"),
    load("./comps/d-item.html"),
    load("./comps/d-nav.html"),
    load("./comps/doc-aside.html"),
    load("./comps/doc-container.html"),
    load("./comps/exm-article.html"),
    load("./comps/lang-select.html"),
    load("./comps/simp-editor.html"),
    load("./layouts/article-layout.html"),
    load("./layouts/header-layout.html"),
    load("./comps/viewer/html-viewer.html"),
    load("./comps/viewer/comp-viewer.html"),
    load("./comps/viewer/files-viewer.html"),
    fetch(resolvePath("./css/github-markdown.css")),
    fetch(resolvePath("./css/public.css")),
    fetch(resolvePath("./comps/article-nav/article-nav-host.css")),
    fetch(resolvePath("./comps/doc-header/doc-header.css")),
  ]).then(() => {
    const loading = $("#loading");
    if (loading) {
      setTimeout(() => {
        loading.classList.add("fadeout");
        setTimeout(() => {
          loading.remove();
        }, 400);
      }, 200);
    }
  });
})();
