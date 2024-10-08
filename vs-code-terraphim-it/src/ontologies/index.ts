
/* -----------------------------------
* GENERATED WITH @tomic/cli
* -------------------------------- */

import { registerOntologies } from '@tomic/lib';

import { airportOntology } from './airportOntology.js';
import { externals } from './externals.js';

export function initOntologies(): void {
  registerOntologies(airportOntology, externals);
}
