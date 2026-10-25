import { mount } from 'svelte';
import '@fontsource-variable/source-serif-4/opsz.css';
import '@fontsource-variable/source-serif-4/opsz-italic.css';
import App from './App.svelte';
import './app.css';

mount(App, { target: document.getElementById('app')! });
