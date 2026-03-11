// Terraphim AI Logo - Mathober Day 10 Variation
// Stellated polygon mandala with inverted triangle motif
// Original concept: radial graph with stellated polygons symmetrically
// rotated to make a mandala. Adapted for Terraphim brand.

let t = 0;
let n = 25;
let g = 6;
let triangleMask;

function setup() {
  let size = min(windowWidth, windowHeight, 500);
  let canvas = createCanvas(size, size);
  canvas.parent("logo-container");
  background(15, 12, 10);
  describe(
    "An animated Terraphim AI logo with stellated polygons forming a mandala pattern in orange and gold, framed within an inverted triangle."
  );
  rectMode(CENTER);
  strokeWeight(0.5);
  strokeCap(SQUARE);
  g = random([5, 6, 7, 8, 10, 12]);
  triangleMask = createGraphics(width, height);
}

function draw() {
  background(15, 12, 10, 20);
  translate(width / 2, height / 2);

  // Radial grid - subtle orange
  stroke(200, 120, 40, 8);
  drawGrid();

  t += 0.05;
  noFill();

  push();
  rotate(t / 100);

  // Inner tight stellations - bright orange
  stellations((width / 25) * 2.5, (width / 25) * sin(t / 10), 0, 5, 1);
  // Core stellations - gold
  stellations((width / 25) * 1.5, (width / 25) * sin(t / 10), 0, 2, 1);
  // Outer wide stellations - darker orange
  stellations((width / 25) * 8, (width / 25) * sin(t / 10), PI / g, 8, 1);

  pop();

  // Draw the inverted triangle frame
  drawTriangleFrame();

  // Outer border
  noFill();
  stroke(200, 120, 40, 30);
  rect(0, 0, width / 1.02, width / 1.02);
  rect(0, 0, width / 1.04, width / 1.04);

  // Terraphim T letterform hint
  drawTMark();
}

function drawGrid() {
  for (let i = 0; i < n; i++) {
    rotate((2 * PI) / n);
    line(-width / 1.5, 0, width / 1.5, 0);
    // Concentric circles in gold
    stroke(245, 197, 24, 5);
    circle(0, 0, (width / n) * 2 * i);
    stroke(200, 120, 40, 8);
  }
  // Static noise particles - warm tones
  noStroke();
  for (let i = 0; i < 400; i++) {
    let x = random(-width / 2, width / 2);
    let y = random(-width / 2, width / 2);
    fill(245, 197, 24, random(30, 80));
    circle(x, y, random(1, 2.5));
  }
}

function stellations(r1, r2, r3, a, b) {
  push();
  rotate(r3);
  for (let j = 0; j < g; j++) {
    rotate(PI / (g / 2));

    // Fill with warm translucent orange
    fill(232, 128, 28, 8);
    beginShape();
    for (let i = 0; i < g; i++) {
      let x = r1 * cos((i * PI) / (g / 2));
      let y = (width / 25) * a + r2 * sin((i * PI) / (g / 2));
      vertex(x, y);
      strokeWeight(0.5);
      // Lines in graduated orange-to-gold
      let orange = lerpColor(
        color(232, 128, 28, 80),
        color(245, 197, 24, 80),
        i / g
      );
      stroke(orange);
      line(x, y, 0, width / 5 + (width / 25) * 8 * sin(t / 10) * cos(t / 15));
    }
    endShape(CLOSE);
  }
  pop();
}

function drawTriangleFrame() {
  // Inverted triangle - the Terraphim symbol
  let s = width * 0.42;
  let topY = -s * 0.45;
  let botY = s * 0.55;
  let halfW = s * 0.55;

  push();
  noFill();

  // Outer triangle - bold orange
  strokeWeight(2.5);
  stroke(232, 128, 28, 180);
  beginShape();
  vertex(-halfW, topY);
  vertex(halfW, topY);
  vertex(0, botY);
  endShape(CLOSE);

  // Inner triangle - thinner gold
  let inset = 0.88;
  strokeWeight(1);
  stroke(245, 197, 24, 120);
  beginShape();
  vertex(-halfW * inset, topY + s * 0.03);
  vertex(halfW * inset, topY + s * 0.03);
  vertex(0, botY * inset);
  endShape(CLOSE);

  // Outer glow triangle
  strokeWeight(0.5);
  let glowAlpha = 40 + 20 * sin(t / 5);
  stroke(245, 197, 24, glowAlpha);
  let outer = 1.08;
  beginShape();
  vertex(-halfW * outer, topY - s * 0.02);
  vertex(halfW * outer, topY - s * 0.02);
  vertex(0, botY * outer + s * 0.02);
  endShape(CLOSE);

  pop();
}

function drawTMark() {
  // Subtle T letterform at center
  let alpha = 60 + 30 * sin(t / 8);
  push();
  strokeWeight(2);
  stroke(245, 197, 24, alpha);
  noFill();

  // T horizontal bar
  let barW = width * 0.08;
  let barY = -width * 0.04;
  line(-barW, barY, barW, barY);

  // T vertical stem
  line(0, barY, 0, barY + width * 0.1);

  pop();
}

function mousePressed() {
  g = random([5, 6, 7, 8, 10, 12]);
}

function windowResized() {
  let size = min(windowWidth, windowHeight, 500);
  resizeCanvas(size, size);
  if (triangleMask) {
    triangleMask.resizeCanvas(size, size);
  }
}
