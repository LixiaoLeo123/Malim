<script lang="ts">
    export let code: string = "KR";
    export let size: number = 20;
</script>

<div
    class="rounded-full overflow-hidden border border-zinc-200 shadow-sm inline-block relative bg-white shrink-0"
    style="width: {size}px; height: {size}px;"
>
    {#if code === "KR" || code === "Korean"}
        <svg
            viewBox="0 0 36 36"
            xmlns="http://www.w3.org/2000/svg"
            class="w-full h-full object-cover"
        >
            <rect fill="#FFFFFF" width="36" height="36" />
            <g transform="rotate(34.8 18 18)">
                <rect x="13" y="3" width="10" height="1" fill="#000" />
                <rect x="13" y="4.5" width="10" height="1" fill="#000" />
                <rect x="13" y="6" width="10" height="1" fill="#000" />
            </g>
            <g transform="rotate(214.8 18 18)">
                <rect x="13" y="3" width="4.5" height="1" fill="#000" />
                <rect x="18.5" y="3" width="4.5" height="1" fill="#000" />
                <rect x="13" y="4.5" width="4.5" height="1" fill="#000" />
                <rect x="18.5" y="4.5" width="4.5" height="1" fill="#000" />
                <rect x="13" y="6" width="4.5" height="1" fill="#000" />
                <rect x="18.5" y="6" width="4.5" height="1" fill="#000" />
            </g>
            <g transform="rotate(268 18 18) translate(0 1)">
                <rect x="12" y="3" width="4.5" height="1" fill="#000" />
                <rect x="17.5" y="3" width="4.5" height="1" fill="#000" />
                <rect x="12" y="4.5" width="10" height="1" fill="#000" />
                <rect x="12" y="6" width="4.5" height="1" fill="#000" />
                <rect x="17.5" y="6" width="4.5" height="1" fill="#000" />
            </g>
            <g transform="rotate(88 18 18) translate(0 1)">
                <rect x="12" y="3" width="10" height="1" fill="#000" />
                <rect x="12" y="4.5" width="4.5" height="1" fill="#000" />
                <rect x="17.5" y="4.5" width="4.5" height="1" fill="#000" />
                <rect x="12" y="6" width="10" height="1" fill="#000" />
            </g>
            <circle cx="18" cy="18" r="9" fill="#FFFFFF" />
            <path
                fill="#CD2E3A"
                d="M18,18l0,-9a9,9 0 0 1 0,18a4.5,4.5 0 0 1 0,-9a4.5,4.5 0 0 0 0,-9"
            />
            <path
                fill="#0047A0"
                d="M18,18l0,9a9,9 0 0 1 0,-18a4.5,4.5 0 0 1 0,9a4.5,4.5 0 0 0 0,9"
            />
        </svg>
    {:else if code === "RU" || code === "Russian"}
        <svg
            viewBox="0 0 900 600"
            xmlns="http://www.w3.org/2000/svg"
            class="w-full h-full object-cover"
        >
            <rect fill="#fff" width="900" height="600" />
            <rect fill="#0039a6" y="200" width="900" height="200" />
            <rect fill="#d52b1e" y="400" width="900" height="200" />
        </svg>
    {:else if code === "JP" || code === "Japanese"}
        <svg
            viewBox="0 0 900 600"
            xmlns="http://www.w3.org/2000/svg"
            class="w-full h-full object-cover"
        >
            <rect fill="#ffffff" width="900" height="600" />
            <circle cx="450" cy="300" r="180" fill="#BC002D" />
        </svg>
    {:else if code === "ES" || code === "Spanish"}
        <svg
            viewBox="0 0 750 500"
            xmlns="http://www.w3.org/2000/svg"
            class="w-full h-full object-cover"
        >
            <!-- Flag background: red-yellow-red -->
            <rect fill="#AA151B" y="0" width="750" height="125" />
            <rect fill="#F1BF00" y="125" width="750" height="250" />
            <rect fill="#AA151B" y="375" width="750" height="125" />
            <!-- Coat of arms on yellow band (centered, offset to hoist side per Spanish protocol) -->
            <g transform="translate(160, 155) scale(0.65)">
                <!-- Shield outline -->
                <rect x="0" y="0" width="220" height="280" rx="12" fill="none" stroke="#00369B" stroke-width="2" />
                <!-- Quarter 1: Castle (Castile) on red -->
                <rect x="2" y="2" width="108" height="138" fill="#AA151B" />
                <g transform="translate(8, 8)" fill="#F1BF00" stroke="#000" stroke-width="0.5">
                    <rect x="10" y="30" width="20" height="35" />
                    <rect x="45" y="30" width="20" height="35" />
                    <rect x="80" y="30" width="20" height="35" />
                    <rect x="10" y="70" width="78" height="25" />
                    <rect x="25" y="15" width="15" height="15" />
                    <rect x="60" y="15" width="15" height="15" />
                </g>
                <!-- Quarter 2: Rampant Lion (León) on white -->
                <rect x="110" y="2" width="108" height="138" fill="#FFFFFF" />
                <g transform="translate(125, 5)" fill="#AA151B">
                    <!-- Stylized lion -->
                    <path d="M30,130 l5,-15 l10,5 l8,-25 l12,10 l8,-30 l10,5 l5,-20 l8,5 l-2,20 l8,0 l5,25 l8,-10 l5,15 l-10,20 l-15,0 l-12,-15 l-15,15 l-10,-5 l-8,10 Z" />
                </g>
                <!-- Quarter 3: Vertical stripes (Aragon) on gold -->
                <rect x="2" y="140" width="108" height="138" fill="#F1BF00" />
                <g>
                    <rect x="10" y="140" width="10" height="138" fill="#AA151B" />
                    <rect x="28" y="140" width="10" height="138" fill="#AA151B" />
                    <rect x="46" y="140" width="10" height="138" fill="#AA151B" />
                    <rect x="64" y="140" width="10" height="138" fill="#AA151B" />
                </g>
                <!-- Quarter 4: Chains (Navarre) on red -->
                <rect x="110" y="140" width="108" height="138" fill="#AA151B" />
                <g stroke="#F1BF00" stroke-width="3" fill="none">
                    <circle cx="164" cy="209" r="25" />
                    <line x1="164" y1="140" x2="164" y2="184" />
                    <line x1="164" y1="234" x2="164" y2="278" />
                    <line x1="110" y1="209" x2="139" y2="209" />
                    <line x1="189" y1="209" x2="218" y2="209" />
                </g>
                <!-- Central inescutcheon: Bourbon fleur-de-lis on blue -->
                <ellipse cx="110" cy="175" rx="35" ry="45" fill="#00369B" stroke="#AA151B" stroke-width="2" />
                <g fill="#F1BF00" transform="translate(95, 150)">
                    <path d="M15,5 l3,12 l10,3 l-7,8 l2,12 l-8,-5 l-8,5 l2,-12 l-7,-8 l10,-3 Z" />
                    <path d="M5,30 l3,10 l9,2 l-6,7 l1,10 l-7,-4 l-7,4 l1,-10 l-6,-7 l9,-2 Z" />
                    <path d="M25,30 l3,10 l9,2 l-6,7 l1,10 l-7,-4 l-7,4 l1,-10 l-6,-7 l9,-2 Z" />
                </g>
                <!-- Crown at top -->
                <g transform="translate(35, -55)" fill="#F1BF00" stroke="#AA151B" stroke-width="1">
                    <rect x="0" y="40" width="150" height="15" rx="3" />
                    <circle cx="15" cy="40" r="12" />
                    <circle cx="40" cy="40" r="12" />
                    <circle cx="65" cy="40" r="12" />
                    <circle cx="90" cy="40" r="12" />
                    <circle cx="115" cy="40" r="12" />
                    <circle cx="135" cy="40" r="12" />
                    <!-- Crown arches -->
                    <rect x="0" y="25" width="150" height="15" rx="3" />
                    <path d="M5,25 Q35,-10 75,25" fill="none" stroke="#F1BF00" stroke-width="4" />
                    <path d="M75,25 Q115,-10 145,25" fill="none" stroke="#F1BF00" stroke-width="4" />
                </g>
                <!-- Pillars of Hercules -->
                <g transform="translate(-40, 100)">
                    <rect x="0" y="0" width="12" height="160" fill="#FFFFFF" stroke="#00369B" stroke-width="1" />
                    <text x="16" y="80" font-size="12" fill="#000">PLVS</text>
                </g>
                <g transform="translate(248, 100)">
                    <rect x="0" y="0" width="12" height="160" fill="#FFFFFF" stroke="#00369B" stroke-width="1" />
                    <text x="-38" y="80" font-size="12" fill="#000">VLTRA</text>
                </g>
            </g>
        </svg>
    {:else}
        <div class="w-full h-full bg-zinc-200"></div>
    {/if}
</div>
