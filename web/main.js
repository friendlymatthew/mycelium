import init, { WasmRenderer } from '../renderer/pkg/renderer.js';

let renderer = null;
let animationId = null;

async function loadGpsData() {
  // Load GPS data from the exported parquet file (relative path for GitHub Pages)
  const response = await fetch('./gps-data.json');
  if (!response.ok) {
    console.error('Failed to load GPS data:', response.status, response.statusText);
    return null;
  }
  console.log('GPS data loaded successfully');
  return await response.json();
}

function animate() {
  if (renderer) {
    try {
      renderer.render();
    } catch (e) {
      console.error('Render error:', e);
    }
  }
  animationId = requestAnimationFrame(animate);
}

function resizeCanvas(canvas) {
  const displayWidth = canvas.clientWidth;
  const displayHeight = canvas.clientHeight;

  // Account for device pixel ratio (retina displays)
  const dpr = window.devicePixelRatio || 1;
  const width = Math.floor(displayWidth * dpr);
  const height = Math.floor(displayHeight * dpr);

  if (canvas.width !== width || canvas.height !== height) {
    canvas.width = width;
    canvas.height = height;

    if (renderer) {
      renderer.resize(width, height);
    }
  }
}

async function main() {
  const canvas = document.getElementById('canvas');
  const loading = document.getElementById('loading');
  const loadingText = document.getElementById('loading-text');
  const loadingProgress = document.getElementById('loading-progress');
  const instructions = document.getElementById('instructions');

  try {
    // Initialize WASM
    loadingText.textContent = 'Initializing WASM...';
    await init();
    console.log('WASM initialized');

    // Set canvas size
    resizeCanvas(canvas);

    // Create renderer
    loadingText.textContent = 'Setting up renderer...';
    renderer = new WasmRenderer(canvas);
    await renderer.initialize();
    console.log('Renderer initialized');

    // Load GPS data
    loadingText.textContent = 'Loading GPS data...';
    loadingProgress.textContent = '(~5MB, may take a moment)';
    const gpsData = await loadGpsData();
    if (gpsData) {
      loadingText.textContent = 'Processing activities...';
      loadingProgress.textContent = `${gpsData.length} activities with ${gpsData.reduce((sum, a) => sum + a.length, 0).toLocaleString()} points`;
      renderer.load_activities(JSON.stringify(gpsData));
      console.log('GPS data loaded');
    }

    // Hide loading, show instructions
    loading.classList.add('hidden');
    instructions.classList.remove('hidden');

    // Start animation loop
    animate();

    // Handle window resize
    window.addEventListener('resize', () => resizeCanvas(canvas));

    // Mouse events
    canvas.addEventListener('mousedown', (e) => {
      renderer.on_mouse_down(e.button, e.clientX, e.clientY);
    });

    canvas.addEventListener('mouseup', (e) => {
      renderer.on_mouse_up(e.button);
    });

    canvas.addEventListener('mousemove', (e) => {
      renderer.on_mouse_move(e.clientX, e.clientY);
    });

    canvas.addEventListener('wheel', (e) => {
      e.preventDefault();
      renderer.on_wheel(e.deltaY);
    }, { passive: false });

    // Prevent context menu on right click
    canvas.addEventListener('contextmenu', (e) => {
      e.preventDefault();
    });

    // Keyboard events
    window.addEventListener('keydown', (e) => {
      renderer.on_key_down(e.key);
    });

  } catch (e) {
    console.error('Failed to initialize:', e);
    loading.innerHTML = `
      <div style="color: #f44;">Error: ${e.message}</div>
      <div style="margin-top: 1rem; font-size: 0.9rem;">
        Make sure to build the WASM module first:<br>
        <code style="display: block; margin-top: 0.5rem; background: #222; padding: 0.5rem; border-radius: 4px;">
          cd renderer && wasm-pack build --target web
        </code>
      </div>
    `;
  }
}

// Cleanup on page unload
window.addEventListener('beforeunload', () => {
  if (animationId) {
    cancelAnimationFrame(animationId);
  }
});

main();
