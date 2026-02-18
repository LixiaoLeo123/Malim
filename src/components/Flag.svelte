<script lang="ts">
    export let code: string = 'KR';
    export let size: number = 20;
</script>

<div 
    class="rounded-full overflow-hidden border border-zinc-200 shadow-sm inline-block relative bg-white shrink-0"
    style="width: {size}px; height: {size}px;"
>
    {#if code === 'KR' || code === 'Korean'}
        <svg viewBox="0 0 36 36" xmlns="http://www.w3.org/2000/svg" class="w-full h-full object-cover">
            <rect fill="#FFFFFF" width="36" height="36"/>
            <g transform="rotate(34.8 18 18)">
                <rect x="13" y="3" width="10" height="1" fill="#000"/>
                <rect x="13" y="4.5" width="10" height="1" fill="#000"/>
                <rect x="13" y="6" width="10" height="1" fill="#000"/>
            </g>
            <g transform="rotate(214.8 18 18)">
                 <rect x="13" y="3" width="4.5" height="1" fill="#000"/> <rect x="18.5" y="3" width="4.5" height="1" fill="#000"/>
                 <rect x="13" y="4.5" width="4.5" height="1" fill="#000"/> <rect x="18.5" y="4.5" width="4.5" height="1" fill="#000"/>
                 <rect x="13" y="6" width="4.5" height="1" fill="#000"/> <rect x="18.5" y="6" width="4.5" height="1" fill="#000"/>
            </g>
            <g transform="rotate(268 18 18) translate(0 1)">
                 <rect x="12" y="3" width="4.5" height="1" fill="#000"/> <rect x="17.5" y="3" width="4.5" height="1" fill="#000"/>
                 <rect x="12" y="4.5" width="10" height="1" fill="#000"/>
                 <rect x="12" y="6" width="4.5" height="1" fill="#000"/> <rect x="17.5" y="6" width="4.5" height="1" fill="#000"/>
            </g>
            <g transform="rotate(88 18 18) translate(0 1)">
                <rect x="12" y="3" width="10" height="1" fill="#000"/>
                <rect x="12" y="4.5" width="4.5" height="1" fill="#000"/> <rect x="17.5" y="4.5" width="4.5" height="1" fill="#000"/>
                <rect x="12" y="6" width="10" height="1" fill="#000"/>
            </g>
            <circle cx="18" cy="18" r="9" fill="#FFFFFF"/>
            <path fill="#CD2E3A" d="M18,18l0,-9a9,9 0 0 1 0,18a4.5,4.5 0 0 1 0,-9a4.5,4.5 0 0 0 0,-9"/>
            <path fill="#0047A0" d="M18,18l0,9a9,9 0 0 1 0,-18a4.5,4.5 0 0 1 0,9a4.5,4.5 0 0 0 0,9"/>
        </svg>
    {:else if code === 'RU' || code === 'Russian'}
        <svg viewBox="0 0 900 600" xmlns="http://www.w3.org/2000/svg" class="w-full h-full object-cover">
            <rect fill="#fff" width="900" height="600"/>
            <rect fill="#0039a6" y="200" width="900" height="200"/>
            <rect fill="#d52b1e" y="400" width="900" height="200"/>
        </svg>
    {:else}
        <div class="w-full h-full bg-zinc-200"></div>
    {/if}
</div>
