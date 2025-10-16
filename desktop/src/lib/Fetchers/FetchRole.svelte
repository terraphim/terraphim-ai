<script lang="ts">
export let subject: string;

import { urls } from '@tomic/lib';
import { getResource, getValue } from '@tomic/svelte';

const resource1 = getResource(subject);

const _name = getValue<string>(resource1, urls.properties.name);
const _theme = getValue<string>(resource1, 'http://localhost:9883/property/theme');
</script>

<option value={subject}>{$name}</option>
<!--     -->
