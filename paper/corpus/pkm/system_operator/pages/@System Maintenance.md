type:: [[article]]
source:: [[@Guide to the Systems Engineering Body of Knowledge (SEBoK version 2.8)]]
topic:: [[Maintenance]]

- **Lead Authors:** ==[[Scott Jackson]] , [[Brian Gallagher]], Contributing Author: [[David Dorgan]]==
- ==System Maintenance== planning begins early in the acquisition process with the development of a ==maintenance concept==. ==Maintenance planning== is conducted to evolve and establish requirements and tasks to be accomplished for achieving, restoring, and ==maintaining operational capability== for the life of the system. For a system to be sustained throughout its system life cycle, the ==maintenance process== has to be executed concurrently with the ==operations process== (ISO/IEC/IEEE 15288:2015, Clause 6.4.9).
- ![image.png](../assets/image_1689762871422_0.png){:height 325, :width 711}
- ## Overview
	- The ==initial requirements for maintenance== have to be defined during the stakeholder needs and requirement definition process (Clause 6.4.1 ISO/IEC/IEEE 15288:2015) and continue to evolve during the development and operation of the system. Considerations include:
		- ![image.png](../assets/image_1689763274010_0.png)
	- Maximizing system availability to meet the operational requirements. This has to take into account the designed-in ==reliability== and ==maintainability== of the system and resources available.
	- Preserving ==system operating potential== through proper planning of ==system scheduled maintenance==. This requires a ==reliability-centered maintenance== strategy that incorporates ==preventive maintenance== in order to preempt failures, thereby extending the mean time between ==corrective maintenance==, as well as enhancing the availability of the system.
	- Segmenting ==maintenance activities== for potential outsourcing of non-critical activities to ==approved maintenance subcontractors== as to optimize scarce technical manpower resources and ==maintenance/repair turn-around times==.
	- Harnessing IT technology for ==maintenance management==. This involves rigorous and systematic ==capturing and tracking of operating and maintenance activities== to facilitate analysis and planning.
	- ==Maintenance management== is concerned with the development and review of ==maintenance plans==, as well as securing and coordinating resources, such as budget, ==service parts provisioning==, and management of supporting tasks (e.g., contract administration, engineering support, and quality assurance). ==Maintenance planning== relies on ==level of repair analysis (LORA)== as a function of the ==system acquisition process==. Initial planning addresses actions and support necessary to ensure a minimum life cycle cost.
- ## Process Approaches
	- The purpose of the ==maintenance process== is to sustain the capability of a system to provide a service. This process monitors the system’s capability to deliver services, records problems for analysis, takes corrective, adaptive, perfective, and preventive actions, and confirms restored capability. As a result of the successful ==implementation of the maintenance process==:
	- a ==maintenance strategy== is developed
	- ==maintenance constraints== are provided as inputs to requirements
	  collapsed:: true
		- This relation is missing in the current [[@Digital Systems Engineering Process Model Version: 1.0]].
		  id:: 64b7d158-67cd-4af4-8880-a58f700beec6
	- ==replacement system elements== are made available
	- ==services meeting stakeholder requirements== are sustained
	- ==the need for corrective design changes== is reported
	- ==failure and lifetime data== are recorded
	- The project should implement the following activities and tasks in accordance with applicable organization policies and procedures with respect to the maintenance process:
	- ==scheduled servicing==, such as ==daily inspection/checks, servicing, and cleaning==
	- ==unscheduled servicing== (carrying out fault detection and isolation to the faulty replaceable unit and replacement of the failed unit)
	- ==re-configuration of the system== for different roles or functions
	- scheduled servicing (higher level scheduled servicing but below depot level)
	- unscheduled servicing (carrying out more complicated fault isolation to the faulty replaceable unit and replacement of the failed unit)
	- ==minor modifications==
	- ==minor damage repairs==
	- ==major scheduled servicing== (e.g., overhaul and corrosion treatment)
	- ==major repairs== (beyond normal removal and replacement tasks)
	- The ==maintenance plan== specifies the scheduled servicing tasks and intervals (preventive maintenance) and the unscheduled servicing tasks (adaptive or corrective maintenance). Tasks in the maintenance plan are allocated to the various ==maintenance agencies==. A ==maintenance allocation chart== is developed to tag the ==maintenance tasks== to the appropriate maintenance agencies. These include: in-service or in-house work centers, approved contractors, affiliated maintenance or repair facilities, original equipment manufacturer (OEMs), etc. The maintenance plan also establishes the requirements for the support resources.
	- Related activities such as ==resource planning, budgeting, performance monitoring, upgrades, longer term supportability, and sustenance== also need to be managed. These activities are planned, managed, and executed over a longer time horizon and they concern the well-being of the system over the entire life cycle.
	- Proper maintenance of the system (including maintenance-free system designs) relies very much on the ==availability of support resources, such as support and test equipment (STE), technical data and documentation, personnel, spares, and facilities==. These have to be factored in during the ==acquisition agreement process==.
- ## Training and Certification
	- Adequate training must be provided for the ==technical personnel maintaining the system==. While initial training may have been provided during the deployment phase, additional personnel may need to be trained to cope with the increased number of systems being fielded, as well as to cater to staff turnover. Timely updates to training materials and trained personnel may be required as part of system upgrades and evolution. It is important to define the ==certification standards== and contract for the training materials as part of the ==supply agreement==.
- ## Practical Considerations
	- The ==organization responsible for maintaining the system== should have clear thresholds established to determine whether a change requested by end users, changes to correct latent defects, or changes required to fulfill the evolving mission are within the ==scope of a maintenance change== or require a more ==formal project== to step through the entire systems engineering life-cycle. Evaluation criteria to make such a decision could include cost, schedule, risk, or criticality characteristics.
		- This relationship with [[Project change requests]] is missing in the model.
		  id:: 64b7d476-6618-4e6c-9a3c-72b3fa20b112
- ## References
	- ### Works Cited
		- ISO/IEC/IEEE. 2015.*[Systems and Software Engineering - System Life Cycle Processes](https://sebokwiki.org/wiki/ISO/IEC/IEEE_15288).*Geneva, Switzerland: International Organization for Standardization (ISO)/International Electrotechnical Commission (IEC), Institute of Electrical and Electronics Engineers (IEEE). [ISO/IEC/IEEE 15288](https://sebokwiki.org/wiki/ISO/IEC/IEEE_15288):2015.
	- ### Primary References
	  collapsed:: true
		- Blanchard, B.S. and W.J. Fabrycky. 2011. *[Systems Engineering and Analysis](https://sebokwiki.org/wiki/Systems_Engineering_and_Analysis),* 5th Edition. Upper Saddle River, NJ, USA: Prentice Hall.
		- DAU. 2010. *[Defense Acquisition Guidebook (DAG)](https://sebokwiki.org/wiki/Defense_Acquisition_Guidebook_(DAG))*. Ft. Belvoir, VA, USA: Defense Acquisition University (DAU)/U.S. Department of Defense.
		- INCOSE. 2012. *[INCOSE Systems Engineering Handbook](https://sebokwiki.org/wiki/INCOSE_Systems_Engineering_Handbook): A Guide for System Life Cycle Processes and Activities*. Version 3.2.2. San Diego, CA, USA: International Council on Systems Engineering (INCOSE), INCOSE-TP-2003-002-03.2.2.
		- Institute of Engineers Singapore. 2009. *[Systems Engineering Body of Knowledge](https://sebokwiki.org/wiki/Systems_Engineering_Body_of_Knowledge_(Singapore)),* Provisional version 2.0. Singapore: Institute of Engineers Singapore.
		  IISO/IEC/IEEE. 2015.*[Systems and Software Engineering - System Life Cycle Processes](https://sebokwiki.org/wiki/ISO/IEC/IEEE_15288).* Geneva, Switzerland: International Organization for Standardization (ISO)/International Electrotechnical Commission (IEC), Institute of Electrical and Electronics Engineers (IEEE).[ISO/IEC/IEEE 15288](https://sebokwiki.org/wiki/ISO/IEC/IEEE_15288):2015.