<script lang="ts">
    import { onDestroy } from "svelte";
    import { invoke } from "@tauri-apps/api/core";
    import { fade, fly } from "svelte/transition";
    import * as THREE from "three";
    import { OrbitControls } from "three/addons/controls/OrbitControls.js";

    export let open = false;

    let container: HTMLDivElement;
    let scene: THREE.Scene,
        camera: THREE.PerspectiveCamera,
        renderer: THREE.WebGLRenderer;
    let controls: OrbitControls, animationId: number;
    let loading = true;
    let sceneReady = false;

    let lineGeometry: THREE.BufferGeometry;
    let lineMaterial: THREE.ShaderMaterial;
    let lineMesh: THREE.LineSegments;
    let lineOpacities: Float32Array;
    let activePulses: Array<{ index: number; life: number; maxLife: number }> = [];

    const isDarkMode =
        document.documentElement.classList.contains("dark") ||
        (window.matchMedia &&
            window.matchMedia("(prefers-color-scheme: dark)").matches);

    const THEME = {
        coreColor: new THREE.Color(isDarkMode ? "#818cf8" : "#4f46e5"),
        outerColor: new THREE.Color(isDarkMode ? "#34d399" : "#10b981"),
        highlightColor: new THREE.Color(isDarkMode ? "#f472b6" : "#e11d48"),
        textColor: isDarkMode ? "#f3f4f6" : "#111827", 
        lineColor: isDarkMode ? new THREE.Color("#818cf8") : new THREE.Color("#6366f1"),
    };

    async function initScene() {
        if (sceneReady) return;
        sceneReady = true;
        
        console.log("Fetching brain data and initializing 3D scene...");
        let words: Array<{ lemma: string; s: number; p: number }> =
            await invoke("get_brain_words");
        loading = false;

        scene = new THREE.Scene();

        camera = new THREE.PerspectiveCamera(
            45,
            container.clientWidth / container.clientHeight,
            0.1,
            1000,
        );
        camera.position.z = 25;

        renderer = new THREE.WebGLRenderer({ alpha: true, antialias: true });
        renderer.setSize(container.clientWidth, container.clientHeight);
        renderer.setPixelRatio(window.devicePixelRatio);
        container.appendChild(renderer.domElement);

        controls = new OrbitControls(camera, renderer.domElement);
        controls.enableDamping = true;
        controls.dampingFactor = 0.05;
        controls.autoRotate = true;
        controls.autoRotateSpeed = 1.5;
        controls.enablePan = false;

        words.sort((a, b) => b.s - a.s);

        const maxTextNodes = 150;
        const shuffledWords = [...words].sort(() => 0.5 - Math.random());
        const topWordsSet = new Set(
            shuffledWords.slice(0, maxTextNodes).map((w) => w.lemma),
        );

        const geometry = new THREE.BufferGeometry();
        const positions = new Float32Array(words.length * 3);
        const colors = new Float32Array(words.length * 3);

        const r_min = 1.5;
        const r_max = 9.0;

        for (let i = 0; i < words.length; i++) {
            const w = words[i];
            const volFraction = i / words.length;
            const r = r_min + (r_max - r_min) * Math.cbrt(volFraction);

            const u = Math.random();
            const v = Math.random();
            const theta = 2 * Math.PI * u;
            const phi = Math.acos(2 * v - 1);

            let x = r * Math.sin(phi) * Math.cos(theta);
            let y = r * Math.sin(phi) * Math.sin(theta) * 0.8;
            let z = r * Math.cos(phi) * 1.1;

            const gap = 0.6;
            if (x > 0) x += gap;
            else x -= gap;

            positions[i * 3] = x;
            positions[i * 3 + 1] = y;
            positions[i * 3 + 2] = z;

            const baseColor = THEME.coreColor.clone().lerp(THEME.outerColor, volFraction);
            if (w.p > 0.7) {
                baseColor.lerp(THEME.highlightColor, (w.p - 0.7) * 3);
            }

            colors[i * 3] = baseColor.r;
            colors[i * 3 + 1] = baseColor.g;
            colors[i * 3 + 2] = baseColor.b;

            if (topWordsSet.has(w.lemma)) {
                const sprite = createTextSprite(w.lemma, w.p, THEME);
                sprite.position.set(x, y, z);
                scene.add(sprite);
            }
        }

        geometry.setAttribute("position", new THREE.BufferAttribute(positions, 3));
        geometry.setAttribute("color", new THREE.BufferAttribute(colors, 3));
        const material = new THREE.PointsMaterial({
            size: 0.3,
            vertexColors: true,
            transparent: true,
            opacity: 0.8,
            sizeAttenuation: true,
        });
        scene.add(new THREE.Points(geometry, material));

        generateNeuralConnections(positions, words.length);

        controls.addEventListener("start", () => {
            controls.autoRotate = false;
        });

        const animate = () => {
            animationId = requestAnimationFrame(animate);
            updateNeuralPulses();
            controls.update();
            renderer.render(scene, camera);
        };
        animate();

        window.addEventListener("resize", handleResize);
    }

    function generateNeuralConnections(positions: Float32Array, count: number) {
        const maxLines = Math.min(count * 8, 1500);
        const tempPositions = new Float32Array(maxLines * 6);
        lineOpacities = new Float32Array(maxLines * 2);
        let lineIdx = 0;

        for (let i = 0; i < count && lineIdx < maxLines; i++) {
            const attempts = Math.random() > 0.5 ? 2 : 1;
            for (let k = 0; k < attempts; k++) {
                if (lineIdx >= maxLines) break;
                
                const step = Math.floor(Math.random() * 50) + 1;
                const j = i + step;
                if (j >= count) continue;

                const x1 = positions[i * 3], y1 = positions[i * 3 + 1], z1 = positions[i * 3 + 2];
                const x2 = positions[j * 3], y2 = positions[j * 3 + 1], z2 = positions[j * 3 + 2];
                
                const dist = Math.sqrt((x2 - x1) ** 2 + (y2 - y1) ** 2 + (z2 - z1) ** 2);
                
                if (dist > 1.5 && dist < 5.0) {
                    const baseIdx = lineIdx * 6;
                    tempPositions[baseIdx] = x1;
                    tempPositions[baseIdx + 1] = y1;
                    tempPositions[baseIdx + 2] = z1;
                    tempPositions[baseIdx + 3] = x2;
                    tempPositions[baseIdx + 4] = y2;
                    tempPositions[baseIdx + 5] = z2;
                    
                    lineOpacities[lineIdx * 2] = 0;
                    lineOpacities[lineIdx * 2 + 1] = 0;
                    lineIdx++;
                }
            }
        }

        const finalPositions = tempPositions.slice(0, lineIdx * 6);
        lineOpacities = lineOpacities.slice(0, lineIdx * 2);

        lineGeometry = new THREE.BufferGeometry();
        lineGeometry.setAttribute("position", new THREE.BufferAttribute(finalPositions, 3));
        lineGeometry.setAttribute("aOpacity", new THREE.BufferAttribute(lineOpacities, 1));

        lineMaterial = new THREE.ShaderMaterial({
            vertexShader: `
                attribute float aOpacity;
                varying float vOpacity;
                void main() {
                    vOpacity = aOpacity;
                    gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
                }
            `,
            fragmentShader: `
                varying float vOpacity;
                uniform vec3 uColor;
                void main() {
                    gl_FragColor = vec4(uColor, vOpacity);
                }
            `,
            uniforms: {
                uColor: { value: THEME.lineColor }
            },
            transparent: true,
            depthWrite: false,
            blending: THREE.AdditiveBlending
        });

        lineMesh = new THREE.LineSegments(lineGeometry, lineMaterial);
        scene.add(lineMesh);
    }


    function updateNeuralPulses() {
        if (Math.random() < 0.15 && activePulses.length < 50) {
            const index = Math.floor(Math.random() * (lineOpacities.length / 2));
            activePulses.push({
                index,
                life: 0,
                maxLife: 20 + Math.random() * 40
            });
        }

        for (let p = activePulses.length - 1; p >= 0; p--) {
            const pulse = activePulses[p];
            pulse.life++;
            const progress = pulse.life / pulse.maxLife;
            
            const opacity = progress < 0.5 
                ? Math.sin(progress * Math.PI) 
                : Math.sin(progress * Math.PI);

            const baseIdx = pulse.index * 2;
            lineOpacities[baseIdx] = opacity * 0.6;
            lineOpacities[baseIdx + 1] = opacity * 0.6;

            if (pulse.life >= pulse.maxLife) {
                lineOpacities[baseIdx] = 0;
                lineOpacities[baseIdx + 1] = 0;
                activePulses.splice(p, 1);
            }
        }

        if (activePulses.length > 0) {
            lineGeometry.attributes.aOpacity.needsUpdate = true;
        }
    }

    function createTextSprite(text: string, p: number, theme: any) {
        const canvas = document.createElement("canvas");
        const ctx = canvas.getContext("2d")!;
        canvas.width = 256;
        canvas.height = 128;

        const baseSize = 14;
        const fontSize = Math.floor(baseSize + p * baseSize);

        ctx.font = `bold ${fontSize}px ui-sans-serif, system-ui, sans-serif`;
        ctx.textAlign = "center";
        ctx.textBaseline = "middle";

        if (!isDarkMode) {
            ctx.strokeStyle = "rgba(255, 255, 255, 0.95)";
            ctx.lineWidth = 4;
            ctx.lineJoin = "round";
            ctx.strokeText(text, canvas.width / 2, canvas.height / 2);
        } else {
            ctx.strokeStyle = "rgba(0, 0, 0, 0.8)";
            ctx.lineWidth = 3;
            ctx.lineJoin = "round";
            ctx.strokeText(text, canvas.width / 2, canvas.height / 2);
        }

        if (p > 0.8) {
            ctx.fillStyle = `#${theme.highlightColor.getHexString()}`;
        } else {
            ctx.fillStyle = isDarkMode ? theme.textColor : "#111827";
        }
        ctx.fillText(text, canvas.width / 2, canvas.height / 2);

        const texture = new THREE.CanvasTexture(canvas);
        texture.minFilter = THREE.LinearFilter;
        const material = new THREE.SpriteMaterial({
            map: texture,
            transparent: true,
            depthTest: false,
        });
        const sprite = new THREE.Sprite(material);

        const scale = 0.02 * fontSize;
        sprite.scale.set(scale * 2, scale, 1);
        return sprite;
    }

    function handleResize() {
        if (!container || !camera || !renderer) return;
        camera.aspect = container.clientWidth / container.clientHeight;
        camera.updateProjectionMatrix();
        renderer.setSize(container.clientWidth, container.clientHeight);
    }

    function cleanupScene() {
        if (animationId) cancelAnimationFrame(animationId);
        window.removeEventListener("resize", handleResize);
        
        if (lineGeometry) lineGeometry.dispose();
        if (lineMaterial) lineMaterial.dispose();
        
        if (renderer) {
            renderer.dispose();
            if (container && renderer.domElement.parentNode === container) {
                container.removeChild(renderer.domElement);
            }
        }
        if (scene) scene.clear();
        
        activePulses = [];
        sceneReady = false;
    }

    $: if (open && container) initScene();
    $: if (!open && sceneReady) cleanupScene();

    onDestroy(() => {
        cleanupScene();
    });
</script>

{#if open}
    <div
        class="fixed inset-0 z-40 bg-black/30 backdrop-blur-sm dark:bg-black/60"
        on:click={() => (open = false)}
        transition:fade={{ duration: 200 }}
        role="presentation"
    ></div>

    <div
        class="fixed z-50 top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[90vw] max-w-4xl h-[80vh] max-h-[800px] bg-white rounded-2xl shadow-2xl overflow-hidden dark:bg-zinc-900 border border-zinc-200 dark:border-zinc-700 flex flex-col"
        transition:fly={{ y: 20, duration: 200 }}
        role="dialog"
        aria-modal="true"
    >
        <div
            class="px-6 py-4 flex justify-between items-center border-b border-zinc-100 dark:border-zinc-800"
        >
            <div>
                <h2
                    class="text-xl font-bold text-zinc-800 dark:text-zinc-100 uppercase tracking-wider text-sm"
                >
                    Cognitive Landscape
                </h2>
                <p class="text-xs text-zinc-500 dark:text-zinc-400 mt-1">
                    Drag to inspect • Scroll to zoom
                </p>
            </div>
            <button
                on:click={() => (open = false)}
                class="text-zinc-400 hover:text-zinc-600 dark:hover:text-zinc-200 transition-colors p-2"
            >
                <svg
                    class="w-5 h-5"
                    fill="none"
                    viewBox="0 0 24 24"
                    stroke="currentColor"
                >
                    <path
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        stroke-width="2"
                        d="M6 18L18 6M6 6l12 12"
                    />
                </svg>
            </button>
        </div>

        <div
            class="relative flex-1 bg-gradient-to-b from-zinc-50 to-zinc-100 dark:from-zinc-900 dark:to-zinc-950"
        >
            {#if loading}
                <div class="absolute inset-0 flex items-center justify-center">
                    <div
                        class="w-8 h-8 border-4 border-indigo-500 border-t-transparent rounded-full animate-spin"
                    ></div>
                </div>
            {/if}

            <div
                bind:this={container}
                class="w-full h-full cursor-grab active:cursor-grabbing outline-none"
            ></div>
        </div>
    </div>
{/if}
