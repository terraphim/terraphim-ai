/* -----------------------------------
 * GENERATED WITH @tomic-cli
 * -------------------------------- */

export const externals = {
  classes: {},
  properties: {
    aircraftClass:
      'https://common.terraphim.io/argu-site/86qcyzy28tk/ontology/airport-ontology/property/aircraft-class',
  },
} as const;

declare module '@tomic/lib' {
  interface PropTypeMapping {
    ['https://common.terraphim.io/argu-site/86qcyzy28tk/ontology/airport-ontology/property/aircraft-class']: string[];
  }

  interface PropSubjectToNameMapping {
    ['https://common.terraphim.io/argu-site/86qcyzy28tk/ontology/airport-ontology/property/aircraft-class']: 'aircraftClass';
  }
}
