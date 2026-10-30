<script lang="ts">
  // The window's own icons (entry icons come from the pack theme).
  let { name, size = 16 }: { name: 'search' | 'clear' | 'chevron' | 'gear' | 'pin' | 'back' | 'grip' | 'check' | 'book'; size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 20 20" aria-hidden="true" fill={name === 'gear' || name === 'grip' ? 'currentColor' : 'none'}
  stroke={name === 'gear' || name === 'grip' ? 'none' : 'currentColor'} stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
  {#if name === 'search'}<circle cx="8.75" cy="8.75" r="5.6"></circle><path d="M13.1 13.1 17.5 17.5"></path>
  {:else if name === 'clear'}<path d="M5.5 5.5l9 9M14.5 5.5l-9 9"></path>
  {:else if name === 'chevron'}<path d="M5.5 8l4.5 4.5L14.5 8"></path>
  {:else if name === 'back'}<path d="M12.5 4.5 7 10l5.5 5.5"></path>
  {:else if name === 'check'}<path d="M4.5 10.5l3.5 3.5 7.5-8"></path>
  {:else if name === 'pin'}<path d="M7.5 2.5h5l-.6 5 3.1 3.1H5l3.1-3.1z"></path><path d="M10 10.6v7"></path>
  {:else if name === 'book'}<path d="M10 5.6C8.4 4.4 6.2 3.9 3.5 4.1v11c2.7-.2 4.9.3 6.5 1.5 1.6-1.2 3.8-1.7 6.5-1.5v-11c-2.7-.2-4.9.3-6.5 1.5z"></path><path d="M10 5.6v11"></path>
  {:else if name === 'grip'}{#each [5, 10, 15] as y}<circle cx="7.5" cy={y} r="1.4"></circle><circle cx="12.5" cy={y} r="1.4"></circle>{/each}
  {:else}<path fill-rule="evenodd" d="M10 6.6a3.4 3.4 0 1 0 0 6.8 3.4 3.4 0 0 0 0-6.8Zm7.2 4.6-1.6.9a5.8 5.8 0 0 1-.6 1.4l.5 1.8-1.4 1.4-1.8-.5c-.4.3-.9.5-1.4.6l-.9 1.6h-2l-.9-1.6a5.8 5.8 0 0 1-1.4-.6l-1.8.5-1.4-1.4.5-1.8a5.8 5.8 0 0 1-.6-1.4l-1.6-.9v-2l1.6-.9c.1-.5.3-1 .6-1.4l-.5-1.8 1.4-1.4 1.8.5c.4-.3.9-.5 1.4-.6l.9-1.6h2l.9 1.6c.5.1 1 .3 1.4.6l1.8-.5 1.4 1.4-.5 1.8c.3.4.5.9.6 1.4l1.6.9v2Z"></path>
  {/if}
</svg>
