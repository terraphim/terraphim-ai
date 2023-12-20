<script lang="ts">
export let subject:string;
import { getResource, getValue } from '@tomic/svelte';
  import { urls } from '@tomic/lib';
const resource1 = getResource(subject);
  
const name = getValue<string>(resource1, urls.properties.name);
const theme = getValue<string>(resource1, "http://localhost:9883/property/theme");

</script>
<option value="{subject}">{$name}</option>
<!--     -->