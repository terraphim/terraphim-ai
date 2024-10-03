
/* -----------------------------------
* GENERATED WITH @tomic/cli
* For more info on how to use ontologies: https://github.com/atomicdata-dev/atomic-server/blob/develop/browser/cli/readme.md
* -------------------------------- */

import type { BaseProps } from '@tomic/lib'

export const airportOntology = {
    classes: {
	landing: 'https://common.terraphim.io/argu-site/86qcyzy28tk/ontology/airport-ontology/class/arrival',
	airport: 'https://common.terraphim.io/argu-site/86qcyzy28tk/ontology/airport-ontology/class/airport',
	aircraft: 'https://common.terraphim.io/argu-site/86qcyzy28tk/ontology/airport-ontology/class/aircraft',
	airline: 'https://common.terraphim.io/argu-site/86qcyzy28tk/ontology/airport-ontology/class/airline',
	gate: 'https://common.terraphim.io/argu-site/86qcyzy28tk/ontology/airport-ontology/class/gate',
	terminal: 'https://common.terraphim.io/argu-site/86qcyzy28tk/ontology/airport-ontology/class/terminal',
	baggageClaimArea: 'https://common.terraphim.io/argu-site/86qcyzy28tk/ontology/airport-ontology/class/baggage-claim-area',
	personnel: 'https://common.terraphim.io/argu-site/86qcyzy28tk/ontology/airport-ontology/class/personnel',
	flightCrew: 'https://common.terraphim.io/argu-site/86qcyzy28tk/ontology/airport-ontology/class/flight-crew',
	weatherConditions: 'https://common.terraphim.io/argu-site/86qcyzy28tk/ontology/airport-ontology/class/weather-conditions',
	runway: 'https://common.terraphim.io/argu-site/86qcyzy28tk/ontology/airport-ontology/class/runway',
	groundServices: 'https://common.terraphim.io/argu-site/86qcyzy28tk/ontology/airport-ontology/class/ground-services',
	flight: 'https://common.terraphim.io/argu-site/86qcyzy28tk/ontology/airport-ontology/class/flight',
	taxiway: 'https://common.terraphim.io/argu-site/86qcyzy28tk/ontology/airport-ontology/class/taxiway',
	aircraftStand: 'https://common.terraphim.io/argu-site/86qcyzy28tk/ontology/airport-ontology/class/parking-spot',
	vehicle: 'https://common.terraphim.io/argu-site/86qcyzy28tk/ontology/airport-ontology/class/vehicle',
	airportSlot: 'https://common.terraphim.io/argu-site/86qcyzy28tk/ontology/airport-ontology/class/airport-slot',
	trafficSeparationService: 'https://common.terraphim.io/argu-site/86qcyzy28tk/ontology/airport-ontology/class/traffic-separation-service',
	takeOffConfiguration: 'https://common.terraphim.io/argu-site/86qcyzy28tk/ontology/airport-ontology/class/take-off-configuration',
	takeOff: 'https://common.terraphim.io/argu-site/86qcyzy28tk/ontology/airport-ontology/class/take-off',
	startUp: 'https://common.terraphim.io/argu-site/86qcyzy28tk/ontology/airport-ontology/class/start-up',
	service: 'https://common.terraphim.io/argu-site/86qcyzy28tk/ontology/airport-ontology/class/service',
	rulesProcedures: 'https://common.terraphim.io/argu-site/86qcyzy28tk/ontology/airport-ontology/class/rules-procedures',
	routeTrajectorySegment: 'https://common.terraphim.io/argu-site/86qcyzy28tk/ontology/airport-ontology/class/route-trajectory-segment',
	organisation: 'https://common.terraphim.io/argu-site/86qcyzy28tk/ontology/airport-ontology/class/organisation-association',
	departureOperations: 'https://common.terraphim.io/argu-site/86qcyzy28tk/ontology/airport-ontology/class/departure-operations',
	departureClearance: 'https://common.terraphim.io/argu-site/86qcyzy28tk/ontology/airport-ontology/class/departure-clearance',
	boarding: 'https://common.terraphim.io/argu-site/86qcyzy28tk/ontology/airport-ontology/class/boarding',
	arrivalOperations: 'https://common.terraphim.io/argu-site/86qcyzy28tk/ontology/airport-ontology/class/arrival-operations',
	apron: 'https://common.terraphim.io/argu-site/86qcyzy28tk/ontology/airport-ontology/class/apron',
	airTrafficControlService: 'https://common.terraphim.io/argu-site/86qcyzy28tk/ontology/airport-ontology/class/air-traffic-control-service',
	airspace: 'https://common.terraphim.io/argu-site/86qcyzy28tk/ontology/airport-ontology/class/airspace',
	aircraftType: 'https://common.terraphim.io/argu-site/86qcyzy28tk/ontology/airport-ontology/class/AircraftType',
	airportSuppliesService: 'https://common.terraphim.io/argu-site/86qcyzy28tk/ontology/airport-ontology/class/airport-supplies-service',
	passengerService: 'https://common.terraphim.io/argu-site/86qcyzy28tk/ontology/airport-ontology/class/passenger-service',
	airsideOperationsDomainLens: 'https://common.terraphim.io/argu-site/86qcyzy28tk/ontology/airport-ontology/class/airside-operations-domain-lens',
	systemOperatorAnalyticalLens: 'https://common.terraphim.io/argu-site/86qcyzy28tk/ontology/airport-ontology/class/system-operator-analytical-lens',
   },
    properties: {
	departureAirport: 'https://common.terraphim.io/argu-site/86qcyzy28tk/ontology/airport-ontology/property/departure-airport',
	code: 'https://common.terraphim.io/argu-site/86qcyzy28tk/ontology/airport-ontology/property/code',
	arrivalTime: 'https://common.terraphim.io/argu-site/86qcyzy28tk/ontology/airport-ontology/property/arrival-time',
	arrivalAirport: 'https://common.terraphim.io/argu-site/86qcyzy28tk/ontology/airport-ontology/property/arrival-airport',
	synonym: 'https://common.terraphim.io/argu-site/86qcyzy28tk/ontology/airport-ontology/property/synonym',
   },
  } as const

export type Landing = typeof airportOntology.classes.landing;
export type Airport = typeof airportOntology.classes.airport;
export type Aircraft = typeof airportOntology.classes.aircraft;
export type Airline = typeof airportOntology.classes.airline;
export type Gate = typeof airportOntology.classes.gate;
export type Terminal = typeof airportOntology.classes.terminal;
export type BaggageClaimArea = typeof airportOntology.classes.baggageClaimArea;
export type Personnel = typeof airportOntology.classes.personnel;
export type FlightCrew = typeof airportOntology.classes.flightCrew;
export type WeatherConditions = typeof airportOntology.classes.weatherConditions;
export type Runway = typeof airportOntology.classes.runway;
export type GroundServices = typeof airportOntology.classes.groundServices;
export type Flight = typeof airportOntology.classes.flight;
export type Taxiway = typeof airportOntology.classes.taxiway;
export type AircraftStand = typeof airportOntology.classes.aircraftStand;
export type Vehicle = typeof airportOntology.classes.vehicle;
export type AirportSlot = typeof airportOntology.classes.airportSlot;
export type TrafficSeparationService = typeof airportOntology.classes.trafficSeparationService;
export type TakeOffConfiguration = typeof airportOntology.classes.takeOffConfiguration;
export type TakeOff = typeof airportOntology.classes.takeOff;
export type StartUp = typeof airportOntology.classes.startUp;
export type Service = typeof airportOntology.classes.service;
export type RulesProcedures = typeof airportOntology.classes.rulesProcedures;
export type RouteTrajectorySegment = typeof airportOntology.classes.routeTrajectorySegment;
export type Organisation = typeof airportOntology.classes.organisation;
export type DepartureOperations = typeof airportOntology.classes.departureOperations;
export type DepartureClearance = typeof airportOntology.classes.departureClearance;
export type Boarding = typeof airportOntology.classes.boarding;
export type ArrivalOperations = typeof airportOntology.classes.arrivalOperations;
export type Apron = typeof airportOntology.classes.apron;
export type AirTrafficControlService = typeof airportOntology.classes.airTrafficControlService;
export type Airspace = typeof airportOntology.classes.airspace;
export type AircraftType = typeof airportOntology.classes.aircraftType;
export type AirportSuppliesService = typeof airportOntology.classes.airportSuppliesService;
export type PassengerService = typeof airportOntology.classes.passengerService;
export type AirsideOperationsDomainLens = typeof airportOntology.classes.airsideOperationsDomainLens;
export type SystemOperatorAnalyticalLens = typeof airportOntology.classes.systemOperatorAnalyticalLens;

declare module '@tomic/lib' {
  interface Classes {
    [airportOntology.classes.landing]: {
    requires: BaseProps;
    recommends: typeof airportOntology.properties.departureAirport | typeof airportOntology.properties.arrivalAirport | typeof airportOntology.properties.synonym;
  };
[airportOntology.classes.airport]: {
    requires: BaseProps | 'https://atomicdata.dev/properties/name' | typeof airportOntology.properties.code | typeof airportOntology.properties.synonym;
    recommends: never;
  };
[airportOntology.classes.aircraft]: {
    requires: BaseProps;
    recommends: typeof airportOntology.properties.synonym;
  };
[airportOntology.classes.airline]: {
    requires: BaseProps;
    recommends: typeof airportOntology.properties.synonym;
  };
[airportOntology.classes.airport]: {
    requires: BaseProps | 'https://atomicdata.dev/properties/name' | typeof airportOntology.properties.code | typeof airportOntology.properties.synonym;
    recommends: never;
  };
[airportOntology.classes.gate]: {
    requires: BaseProps;
    recommends: typeof airportOntology.properties.synonym;
  };
[airportOntology.classes.terminal]: {
    requires: BaseProps;
    recommends: typeof airportOntology.properties.synonym;
  };
[airportOntology.classes.baggageClaimArea]: {
    requires: BaseProps;
    recommends: typeof airportOntology.properties.synonym;
  };
[airportOntology.classes.personnel]: {
    requires: BaseProps;
    recommends: typeof airportOntology.properties.synonym;
  };
[airportOntology.classes.flightCrew]: {
    requires: BaseProps;
    recommends: typeof airportOntology.properties.synonym;
  };
[airportOntology.classes.weatherConditions]: {
    requires: BaseProps;
    recommends: typeof airportOntology.properties.synonym;
  };
[airportOntology.classes.runway]: {
    requires: BaseProps;
    recommends: typeof airportOntology.properties.synonym;
  };
[airportOntology.classes.groundServices]: {
    requires: BaseProps;
    recommends: typeof airportOntology.properties.synonym;
  };
[airportOntology.classes.flight]: {
    requires: BaseProps;
    recommends: typeof airportOntology.properties.synonym;
  };
[airportOntology.classes.taxiway]: {
    requires: BaseProps;
    recommends: typeof airportOntology.properties.synonym;
  };
[airportOntology.classes.aircraftStand]: {
    requires: BaseProps;
    recommends: typeof airportOntology.properties.synonym;
  };
[airportOntology.classes.vehicle]: {
    requires: BaseProps;
    recommends: typeof airportOntology.properties.synonym;
  };
[airportOntology.classes.airportSlot]: {
    requires: BaseProps;
    recommends: typeof airportOntology.properties.synonym;
  };
[airportOntology.classes.trafficSeparationService]: {
    requires: BaseProps;
    recommends: typeof airportOntology.properties.synonym;
  };
[airportOntology.classes.takeOffConfiguration]: {
    requires: BaseProps;
    recommends: typeof airportOntology.properties.synonym;
  };
[airportOntology.classes.takeOff]: {
    requires: BaseProps;
    recommends: typeof airportOntology.properties.synonym;
  };
[airportOntology.classes.startUp]: {
    requires: BaseProps;
    recommends: typeof airportOntology.properties.synonym;
  };
[airportOntology.classes.service]: {
    requires: BaseProps;
    recommends: typeof airportOntology.properties.synonym;
  };
[airportOntology.classes.rulesProcedures]: {
    requires: BaseProps;
    recommends: typeof airportOntology.properties.synonym;
  };
[airportOntology.classes.routeTrajectorySegment]: {
    requires: BaseProps;
    recommends: typeof airportOntology.properties.synonym;
  };
[airportOntology.classes.organisation]: {
    requires: BaseProps;
    recommends: typeof airportOntology.properties.synonym;
  };
[airportOntology.classes.departureOperations]: {
    requires: BaseProps;
    recommends: typeof airportOntology.properties.synonym;
  };
[airportOntology.classes.departureClearance]: {
    requires: BaseProps;
    recommends: typeof airportOntology.properties.synonym;
  };
[airportOntology.classes.boarding]: {
    requires: BaseProps;
    recommends: typeof airportOntology.properties.synonym;
  };
[airportOntology.classes.arrivalOperations]: {
    requires: BaseProps;
    recommends: typeof airportOntology.properties.synonym;
  };
[airportOntology.classes.apron]: {
    requires: BaseProps;
    recommends: typeof airportOntology.properties.synonym;
  };
[airportOntology.classes.airTrafficControlService]: {
    requires: BaseProps;
    recommends: typeof airportOntology.properties.synonym;
  };
[airportOntology.classes.airspace]: {
    requires: BaseProps;
    recommends: typeof airportOntology.properties.synonym;
  };
[airportOntology.classes.aircraftType]: {
    requires: BaseProps | 'https://common.terraphim.io/argu-site/86qcyzy28tk/ontology/airport-ontology/property/aircraft-class';
    recommends: typeof airportOntology.properties.synonym;
  };
[airportOntology.classes.airportSuppliesService]: {
    requires: BaseProps;
    recommends: typeof airportOntology.properties.synonym;
  };
[airportOntology.classes.passengerService]: {
    requires: BaseProps;
    recommends: typeof airportOntology.properties.synonym;
  };
[airportOntology.classes.airsideOperationsDomainLens]: {
    requires: BaseProps | 'https://atomicdata.dev/properties/name' | 'https://atomicdata.dev/properties/description';
    recommends: typeof airportOntology.properties.synonym;
  };
[airportOntology.classes.systemOperatorAnalyticalLens]: {
    requires: BaseProps | 'https://atomicdata.dev/properties/name' | 'https://atomicdata.dev/properties/description';
    recommends: typeof airportOntology.properties.synonym;
  };
  }

  interface PropTypeMapping {
    [airportOntology.properties.departureAirport]: string[]
[airportOntology.properties.code]: string
[airportOntology.properties.arrivalTime]: string
[airportOntology.properties.arrivalAirport]: string[]
[airportOntology.properties.synonym]: string
  }

  interface PropSubjectToNameMapping {
    [airportOntology.properties.departureAirport]: 'departureAirport',
[airportOntology.properties.code]: 'code',
[airportOntology.properties.arrivalTime]: 'arrivalTime',
[airportOntology.properties.arrivalAirport]: 'arrivalAirport',
[airportOntology.properties.synonym]: 'synonym',
  }
}
