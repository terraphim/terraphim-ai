/* generated by Svelte v3.59.2 */
import {
	SvelteComponent,
	append,
	append_styles,
	attr,
	component_subscribe,
	detach,
	element,
	globals,
	init,
	insert,
	listen,
	noop,
	run_all,
	safe_not_equal,
	set_data,
	set_input_value,
	space,
	text
} from "svelte/internal";

const { document: document_1 } = globals;
import { invoke } from "@tauri-apps/api/tauri";
import { open } from "@tauri-apps/api/dialog";
import { onMount } from "svelte";
import { readDir } from "@tauri-apps/api/fs";
import { resolve, appDir, appDataDir } from "@tauri-apps/api/path";
import { isInitialSetupComplete, theme } from "$lib/stores";
import { readBinaryFile } from '@tauri-apps/api/fs';

import {
	register as registerShortcut,
	unregisterAll as unregisterAllShortcuts,
	unregister as unregisterShortcut
} from "@tauri-apps/api/globalShortcut";

import { appWindow } from "@tauri-apps/api/window";

function add_css(target) {
	append_styles(target, "svelte-1v50vlg", ".startup-screen.svelte-1v50vlg{display:flex;align-items:center;justify-content:center;height:100vh}.container.svelte-1v50vlg{max-width:500px}");
}

// (144:4) {#if error}
function create_if_block(ctx) {
	let p;
	let t;

	return {
		c() {
			p = element("p");
			t = text(/*error*/ ctx[2]);
			attr(p, "class", "help is-danger");
		},
		m(target, anchor) {
			insert(target, p, anchor);
			append(p, t);
		},
		p(ctx, dirty) {
			if (dirty & /*error*/ 4) set_data(t, /*error*/ ctx[2]);
		},
		d(detaching) {
			if (detaching) detach(p);
		}
	};
}

function create_fragment(ctx) {
	let meta;
	let meta_content_value;
	let link;
	let link_href_value;
	let t0;
	let div7;
	let div6;
	let h1;
	let t2;
	let p;
	let t4;
	let div1;
	let label0;
	let t6;
	let div0;
	let input0;
	let t7;
	let div3;
	let label1;
	let t9;
	let div2;
	let input1;
	let t10;
	let t11;
	let div5;
	let div4;
	let button;
	let mounted;
	let dispose;
	let if_block = /*error*/ ctx[2] && create_if_block(ctx);

	return {
		c() {
			meta = element("meta");
			link = element("link");
			t0 = space();
			div7 = element("div");
			div6 = element("div");
			h1 = element("h1");
			h1.textContent = "Welcome to Terraphim AI";
			t2 = space();
			p = element("p");
			p.textContent = "Please set up your initial settings:";
			t4 = space();
			div1 = element("div");
			label0 = element("label");
			label0.textContent = "Data Folder Path:";
			t6 = space();
			div0 = element("div");
			input0 = element("input");
			t7 = space();
			div3 = element("div");
			label1 = element("label");
			label1.textContent = "Global Shortcut:";
			t9 = space();
			div2 = element("div");
			input1 = element("input");
			t10 = space();
			if (if_block) if_block.c();
			t11 = space();
			div5 = element("div");
			div4 = element("div");
			button = element("button");
			button.textContent = "Save Settings";
			attr(meta, "name", "color-scheme");

			attr(meta, "content", meta_content_value = /*$theme*/ ctx[3] == "spacelab"
			? "lumen darkly"
			: /*$theme*/ ctx[3]);

			attr(link, "rel", "stylesheet");
			attr(link, "href", link_href_value = `/assets/bulmaswatch/${/*$theme*/ ctx[3]}/bulmaswatch.min.css`);
			attr(h1, "class", "title is-2");
			attr(p, "class", "subtitle");
			attr(label0, "class", "label");
			attr(label0, "for", "data-folder");
			attr(input0, "class", "input");
			attr(input0, "id", "data-folder");
			attr(input0, "type", "text");
			input0.readOnly = true;
			attr(input0, "placeholder", "Click to set path");
			attr(div0, "class", "control");
			attr(div1, "class", "field");
			attr(label1, "class", "label");
			attr(label1, "for", "global-shortcut");
			attr(input1, "class", "input");
			attr(input1, "id", "global-shortcut");
			attr(input1, "type", "text");
			input1.readOnly = true;
			attr(input1, "placeholder", "Click to set shortcut");
			attr(div2, "class", "control");
			attr(div3, "class", "field");
			attr(button, "class", "button is-success");
			attr(div4, "class", "control");
			attr(div5, "class", "field");
			attr(div6, "class", "container svelte-1v50vlg");
			attr(div7, "class", "startup-screen section svelte-1v50vlg");
		},
		m(target, anchor) {
			append(document_1.head, meta);
			append(document_1.head, link);
			insert(target, t0, anchor);
			insert(target, div7, anchor);
			append(div7, div6);
			append(div6, h1);
			append(div6, t2);
			append(div6, p);
			append(div6, t4);
			append(div6, div1);
			append(div1, label0);
			append(div1, t6);
			append(div1, div0);
			append(div0, input0);
			set_input_value(input0, /*dataFolder*/ ctx[0]);
			append(div6, t7);
			append(div6, div3);
			append(div3, label1);
			append(div3, t9);
			append(div3, div2);
			append(div2, input1);
			set_input_value(input1, /*globalShortcut*/ ctx[1]);
			append(div6, t10);
			if (if_block) if_block.m(div6, null);
			append(div6, t11);
			append(div6, div5);
			append(div5, div4);
			append(div4, button);

			if (!mounted) {
				dispose = [
					listen(input0, "input", /*input0_input_handler*/ ctx[7]),
					listen(input0, "click", /*selectFolder*/ ctx[4]),
					listen(input1, "input", /*input1_input_handler*/ ctx[8]),
					listen(input1, "click", /*startCapturingShortcut*/ ctx[5]),
					listen(button, "click", /*saveSettings*/ ctx[6])
				];

				mounted = true;
			}
		},
		p(ctx, [dirty]) {
			if (dirty & /*$theme*/ 8 && meta_content_value !== (meta_content_value = /*$theme*/ ctx[3] == "spacelab"
			? "lumen darkly"
			: /*$theme*/ ctx[3])) {
				attr(meta, "content", meta_content_value);
			}

			if (dirty & /*$theme*/ 8 && link_href_value !== (link_href_value = `/assets/bulmaswatch/${/*$theme*/ ctx[3]}/bulmaswatch.min.css`)) {
				attr(link, "href", link_href_value);
			}

			if (dirty & /*dataFolder*/ 1 && input0.value !== /*dataFolder*/ ctx[0]) {
				set_input_value(input0, /*dataFolder*/ ctx[0]);
			}

			if (dirty & /*globalShortcut*/ 2 && input1.value !== /*globalShortcut*/ ctx[1]) {
				set_input_value(input1, /*globalShortcut*/ ctx[1]);
			}

			if (/*error*/ ctx[2]) {
				if (if_block) {
					if_block.p(ctx, dirty);
				} else {
					if_block = create_if_block(ctx);
					if_block.c();
					if_block.m(div6, t11);
				}
			} else if (if_block) {
				if_block.d(1);
				if_block = null;
			}
		},
		i: noop,
		o: noop,
		d(detaching) {
			detach(meta);
			detach(link);
			if (detaching) detach(t0);
			if (detaching) detach(div7);
			if (if_block) if_block.d();
			mounted = false;
			run_all(dispose);
		}
	};
}

function instance($$self, $$props, $$invalidate) {
	let $theme;
	component_subscribe($$self, theme, $$value => $$invalidate(3, $theme = $$value));
	let dataFolder = "";
	let globalShortcut = "CmdOrControl+X";
	let error = "";
	let isCapturingShortcut = false;

	async function selectFolder() {
		try {
			const selected = await open({
				directory: true,
				multiple: false,
				defaultPath: await appDataDir()
			});

			console.log(selected);
			console.log(typeof selected);

			if (selected && typeof selected === "string") {
				$$invalidate(0, dataFolder = selected);
			} else {
				$$invalidate(2, error = "No folder selected or invalid selection");
			}
		} catch(err) {
			console.error("Failed to open folder selector:", err);
			$$invalidate(2, error = `Failed to open folder selector: ${err.message}`);
		}
	}

	function startCapturingShortcut() {
		isCapturingShortcut = true;
		$$invalidate(1, globalShortcut = "Press your desired shortcut...");
	}

	function handleKeyDown(event) {
		if (!isCapturingShortcut) return;
		event.preventDefault();
		const key = event.key.toUpperCase();
		const modifiers = [];
		if (event.ctrlKey) modifiers.push("Ctrl");
		if (event.altKey) modifiers.push("Alt");
		if (event.shiftKey) modifiers.push("Shift");
		if (event.metaKey) modifiers.push("Cmd");

		if (key !== "CONTROL" && key !== "ALT" && key !== "SHIFT" && key !== "META") {
			$$invalidate(1, globalShortcut = [...modifiers, key].join("+"));
			isCapturingShortcut = false;
		}
	}

	async function saveSettings() {
		// Register the global shortcut
		try {
			await registerShortcut(globalShortcut, () => {
				if (appWindow.isVisible()) {
					appWindow.hide();
				}
			});

			console.log(`Global shortcut ${globalShortcut} registered successfully`);
		} catch(err) {
			$$invalidate(2, error = `Failed to register global shortcut: ${err.message}`);
			console.error("Failed to register global shortcut:", err);
			return;
		}

		if (!dataFolder || !globalShortcut) {
			$$invalidate(2, error = "Please fill in both fields");
			return;
		}

		try {
			await invoke("save_initial_settings", {
				newSettings: {
					data_folder: dataFolder,
					global_shortcut: globalShortcut
				}
			});

			alert("Settings saved successfully");
			await invoke("close_splashscreen");
		} catch(e) {
			$$invalidate(2, error = "Failed to save settings");
			console.error(e);
		} finally {
			// set initial setup complete to true
			isInitialSetupComplete.set(true);
		}
	}

	onMount(() => {
		// unregisterAllShortcuts();
		document.addEventListener("keydown", handleKeyDown);

		return () => {
			document.removeEventListener("keydown", handleKeyDown);
		};
	});

	function input0_input_handler() {
		dataFolder = this.value;
		$$invalidate(0, dataFolder);
	}

	function input1_input_handler() {
		globalShortcut = this.value;
		$$invalidate(1, globalShortcut);
	}

	return [
		dataFolder,
		globalShortcut,
		error,
		$theme,
		selectFolder,
		startCapturingShortcut,
		saveSettings,
		input0_input_handler,
		input1_input_handler
	];
}

class SplashScreen extends SvelteComponent {
	constructor(options) {
		super();
		init(this, options, instance, create_fragment, safe_not_equal, {}, add_css);
	}
}

export default SplashScreen;