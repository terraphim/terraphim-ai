(function () {
  'use strict';

  var prefersReducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;

  var warpCanvas = document.getElementById('warpCanvas');
  var ctx = warpCanvas ? warpCanvas.getContext('2d') : null;
  var stars = [];
  var STAR_COUNT = window.innerWidth < 600 ? 200 : 400;
  var warpSpeed = 0;
  var scrollProgress = 0;

  function resizeCanvas() {
    if (!warpCanvas) return;
    warpCanvas.width = window.innerWidth;
    warpCanvas.height = window.innerHeight;
  }

  function createStar() {
    return {
      x: (Math.random() - 0.5) * warpCanvas.width * 3,
      y: (Math.random() - 0.5) * warpCanvas.height * 3,
      z: Math.random() * 1500 + 200,
      prevX: 0,
      prevY: 0
    };
  }

  function initStars() {
    stars = [];
    for (var i = 0; i < STAR_COUNT; i++) {
      stars.push(createStar());
    }
  }

  function renderStarField() {
    if (prefersReducedMotion || !ctx) return;

    var w = warpCanvas.width;
    var h = warpCanvas.height;
    var cx = w / 2;
    var cy = h / 2;

    ctx.fillStyle = 'rgba(10, 12, 20, 0.25)';
    ctx.fillRect(0, 0, w, h);

    var targetSpeed = 2 + scrollProgress * 30;
    warpSpeed += (targetSpeed - warpSpeed) * 0.05;

    for (var i = 0; i < stars.length; i++) {
      var s = stars[i];

      s.prevX = cx + (s.x / s.z) * 400;
      s.prevY = cy + (s.y / s.z) * 400;

      s.z -= warpSpeed;

      if (s.z < 1) {
        stars[i] = createStar();
        stars[i].z = 1500;
        continue;
      }

      var sx = cx + (s.x / s.z) * 400;
      var sy = cy + (s.y / s.z) * 400;

      var brightness = 1 - s.z / 1700;
      var alpha = Math.max(0, brightness * 0.8);

      if (warpSpeed > 5) {
        var streakAlpha = Math.min(alpha * (warpSpeed / 20), 0.6);
        ctx.beginPath();
        ctx.moveTo(s.prevX, s.prevY);
        ctx.lineTo(sx, sy);
        ctx.strokeStyle = 'rgba(160, 200, 255, ' + streakAlpha + ')';
        ctx.lineWidth = brightness * 1.5;
        ctx.stroke();
      }

      var radius = brightness * 1.8;
      ctx.beginPath();
      ctx.arc(sx, sy, radius, 0, Math.PI * 2);
      ctx.fillStyle = 'rgba(200, 220, 255, ' + alpha + ')';
      ctx.fill();
    }

    requestAnimationFrame(renderStarField);
  }

  if (!prefersReducedMotion && warpCanvas) {
    resizeCanvas();
    initStars();
    renderStarField();

    window.addEventListener('resize', function () {
      resizeCanvas();
    }, { passive: true });

    window.addEventListener('scroll', function () {
      var docHeight = document.documentElement.scrollHeight - window.innerHeight;
      scrollProgress = docHeight > 0 ? window.scrollY / docHeight : 0;
    }, { passive: true });
  }
})();
