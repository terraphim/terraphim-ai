---
created: 2023-09-21T15:57:36 (UTC +01:00)
tags: []
source: https://www.sciencedirect.com/science/article/pii/S0029801823001506
author: S.I. Ahn, R.E. Kurt
---

# Developing an advanced reliability analysis framework for marine systems operations and maintenance - ScienceDirect

> ## Excerpt
> Power generation system reliability is of great concern for all ship operators irrespective of sector, as it provides the highest utility and ensures …

---
[![Elsevier](https://sdfestaticassets-eu-west-1.sciencedirectassets.com/prod/c5ec5024630bc984ae859b0b2315edad4a342b5a/image/elsevier-non-solus.png)](https://www.sciencedirect.com/journal/ocean-engineering "Go to Ocean Engineering on ScienceDirect")

[![Ocean Engineering](https://ars.els-cdn.com/content/image/1-s2.0-S0029801823X00043-cov150h.gif)](https://www.sciencedirect.com/journal/ocean-engineering/vol/272/suppl/C)

[https://doi.org/10.1016/j.oceaneng.2023.113766](https://doi.org/10.1016/j.oceaneng.2023.113766 "Persistent link using digital object identifier")[Get rights and content](https://s100.copyright.com/AppDispatchServlet?publisherName=ELS&contentID=S0029801823001506&orderBeanReset=true)

## Highlights

-   •
    
    Most critical component failures were associated to cooling system.
    
-   •
    
    The lubricating system has the highest reliability of all the systems failure.
    
-   •
    
    The cooling system has the lowest reliability.
    
-   •
    
    Sea chest blockage and scale build-up among major faults.
    

## Abstract

[Power generation system](https://www.sciencedirect.com/topics/engineering/power-generation-system "Learn more about Power generation system from ScienceDirect's AI-generated Topic Pages") reliability is of great concern for all ship operators irrespective of sector, as it provides the highest utility and ensures collective safety of operators, passengers, equipment and cargo. A novel approach to system reliability analysis using DFTA, FMECA and BBN applied to 4 DGs has been conducted. The outcomes provide insight on faults and component [criticality](https://www.sciencedirect.com/topics/engineering/criticality "Learn more about criticality from ScienceDirect's AI-generated Topic Pages") to vessel maintenance and availability. Building from the understanding of how multiple factors can influenced maintenance in addition to routine or age-related wear and tear of machinery. This research looked into operators’ peculiar challenges regarding environment, operational demands and [technology](https://www.sciencedirect.com/topics/earth-and-planetary-sciences/science-and-technology "Learn more about technology from ScienceDirect's AI-generated Topic Pages") challenges that affects maintenance and system reliability. A framework based on outputs from DFTA [minimal cut set](https://www.sciencedirect.com/topics/engineering/minimal-cut-set "Learn more about minimal cut set from ScienceDirect's AI-generated Topic Pages"), RPN from FMECA were used as inputs for BBN to analyse ship marine DG system availability. A BBN influence diagram was used to build a maintenance strategy DSS. Overall outcome for the maintenance strategy selection DSS indicates relatively high unavailability. Therefore, DGs with low availability were recommended to be on Corrective action and ConMon, while DGs with good availability were recommended to be on PMS.

-   [Previous](https://www.sciencedirect.com/science/article/pii/S0029801823001257)
-   [Next](https://www.sciencedirect.com/science/article/pii/S0029801823001774)

## Keywords

System reliability

Maintenance

Dynamic fault tree analysis

Bayesian belief network

Decision support system

Mission critical component

Marine systems

## Nomenclature

ABS(NS)

American Bureau of Shipping (Nautical System)

BBN

Bayesian Belief Network

BE

Basic Event

BSI

[British Standards Institution](https://www.sciencedirect.com/topics/engineering/british-standard-institution "Learn more about British Standards Institution from ScienceDirect's AI-generated Topic Pages")

CBM

Condition Based Maintenance

CMMS

Computerised Maintenance Management System

CPT

Conditional Probability Table

DFTA

Dynamic Fault Tree

DG

Diesel Generator

DNV

Det Norske Veritas

DSS

Decision Support System

EMS

Enterprise Management System

ETA

Event Tree Analysis

FDEP

Functional Dependency

FMEA

[Failure Mode and Effect Analysis](https://www.sciencedirect.com/topics/engineering/failure-mode-and-effect-analysis "Learn more about Failure Mode and Effect Analysis from ScienceDirect's AI-generated Topic Pages")

FMECA

Failure Mode Effect and Criticality Analysis

FTA

Fault Tree Analysis

IM

Importance Measure

IMO

[International Maritime Organisation](https://www.sciencedirect.com/topics/engineering/international-maritime-organisation "Learn more about International Maritime Organisation from ScienceDirect's AI-generated Topic Pages")

ISM code

International Safety Management

LED

Light-emitting diode

MCS

Minimal Cut Set

MDT

Mean Down Time

MRO

Maintenance Repair and Overhaul

MTTF

Mean Time to Failure

NASA

National Aeronautics and Space Administration

NPRD

Non-Electronic Reliability Data

NSWC

Naval Surface [Warfare](https://www.sciencedirect.com/topics/earth-and-planetary-sciences/warfare "Learn more about Warfare from ScienceDirect's AI-generated Topic Pages") Centre

NUREG

Nuclear Regulatory Report

OEM

Original Equipment Manufacturer

OPV

Offshore Patrol Vessel

OREDA

offshore and Onshore Reliability Data

PAND

Priority- AND

PMS

Planned Maintenance System

RCM

Reliability Centred Maintenance

SEQ

Sequence Enforcing

## 1\. Introduction

The foundation of system reliability rests on two primary pillars, the first of which is intrinsic to the system's architecture and the second is obtained via maintenance strategy and execution. The capacity of the operator to effectively follow the maintenance plan set by an organisation helps to reduce failure and maintain the system's excellent operating condition between maintenance intervals. In this respect, the maintenance department's objective will be to use all available strategies to guarantee that failures are not only minimised, but also handled in an efficient and timely way. This might imply efficient maintenance, such as utilising the proper [lubricants](https://www.sciencedirect.com/topics/earth-and-planetary-sciences/lubricating-oil "Learn more about lubricants from ScienceDirect's AI-generated Topic Pages"), changing filters when due or depending on their condition, and ensuring that spare parts inventory reflects component failure and replacement rate. Moreover, the International Safety Management (ISM) code provides guidance to all ship operators, and the document underlined the necessity for companies and ship operators to develop additional requirements to assist effective ship system maintenance ([IACS,2018](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib70)). In addition, it requires ship operators to identify equipment and technical systems whose sudden failure might lead to dangerous circumstances (IMO, 2018).

The existing maintenance strategy namely Corrective, Predictive, Preventive and Condition based maintenance are often selectively combined using reliability analysis procedures to come up with other hybrid maintenance strategies([Lazakis et al., 2018](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib43)). In particular, the wide application of Planned Maintenance System (PMS) and Reliability Centred Maintenance (RCM) in the industry can be attributed to adaptation of more than one maintenance approach to provide maintenance for entire systems ([Karatuğ and Arslanoğlu, 2022](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib38)). In general maintenance guidelines for machinery and equipment forming a system are obtained from the [Original Equipment Manufacturers](https://www.sciencedirect.com/topics/engineering/original-equipment-manufacturer "Learn more about Original Equipment Manufacturers from ScienceDirect's AI-generated Topic Pages") (OEM) Manual. Similarly, alternative elaborate maintenance procedures are equally provided by Classification societies through guidance notes which provides relevant options and information to operators who desire such services (ABS, 2016; IACS, 2021). Moreover, maintenance management systems such as Computerised Maintenance Management System (CMMS) that are stand-alone or come as a part of Enterprise Management System (EMS) are also provided by Classification Societies, Asset management companies, or the maintenance division of the organisation([Eriksen et al., 2021](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib27)). For instance, American Bureau of Shipping (ABS) and DNV-GL provides leading edge maintenance management system such as NS maintenance management and ship manager software respectively(ABS, 2016).

Overall, there are multiple channels which ship operators can access [operation and maintenance](https://www.sciencedirect.com/topics/engineering/operation-and-maintenance "Learn more about operation and maintenance from ScienceDirect's AI-generated Topic Pages") support except that most of this support being enterprise and generic in nature may not be as dynamic and responsive as the operator would want. Hence the need for researchers to provides solutions to both regulators and operators. Maintenance as defined by ([BS, 2010](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib10)) is the combination of all technical, administrative and managerial actions during the life cycle of an item intended to retain it in, or restore it to, a state in which it can perform the required function. Therefore, by emphasising key phrases in the definition, "retain" and "restore"; refer to maintenance activities, while "perform" and "function" refer to the utility needed from the system or equipment. It follows that all maintenance procedures would be designed to guarantee that a system is available at all times within acceptable reliability limits. These limits are defined by factors which largely depends on the reliability inherent to the equipment and its usage. Other factors are to do with operators' environment, maintenance staff competencies, spare parts sourcing and maintenance strategy employed. In this regard the OREDA handbook provides a good reference on multiple equipment reliability analysis approach with acceptable generalisation regarding operating environment and equipment specific reliabilities ([Lazakis et al., 2018](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib43); [Marving Rausand and Arnljot, 2021](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib47)).

In this regard this paper will present the development of an advanced reliability analysis framework for marine systems operations and maintenance through application of reliability analysis tools for identifying maintenance critical components. The methodology also provides a maintenance decision support system for maintenance critical components on board ships. Considering this, the work is organised into parts, with Section [2](https://www.sciencedirect.com/science/article/pii/S0029801823001506#sec2) providing a critical evaluation of system reliability analysis tools, Section [3](https://www.sciencedirect.com/science/article/pii/S0029801823001506#sec3) presenting the innovative approach of this research, and Section [4](https://www.sciencedirect.com/science/article/pii/S0029801823001506#sec4) discussing the case study application of the technique. Section [5](https://www.sciencedirect.com/science/article/pii/S0029801823001506#sec5) presents Results and Discussion, while Section [6](https://www.sciencedirect.com/science/article/pii/S0029801823001506#sec6) presents Conclusions and Recommendations for Future Research.

## 2\. Critical review on system reliability analysis

System reliability analysis is central to the successful implementation of any maintenance strategy as it provides clear insight on machinery behaviour and the impacts of failure on availability of machineries up to system levels ([Ahn et al., 2022](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib2); [Bahoo et al., 2022a](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib5); [Daya and Lazakis, 2022](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib21)). Accordingly, reliability analysis tools are widely used to support maintenance strategy selection or implementation in line with organisational objectives. Therefore, various maintenance strategy such as Reliability Centred Maintenance, Risk based Maintenance, Total Productive Maintenance, Risk and Reliability Based Maintenance etc draw from existing maintenance approach using system reliability analysis to provide a tailored maintenance system([Cheliotis et al., 2020](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib13)). RCM developed in the aviation industry and [United States Navy](https://www.sciencedirect.com/topics/engineering/united-states-navy "Learn more about United States Navy from ScienceDirect's AI-generated Topic Pages") in the 1970s ([NAVSEA, 2007](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib51)) provides clear intersection on the combination of various maintenance strategy and used of reliability tools. For instance, the guidelines for the development and implementation of RCM by the Royal Navy and United States Navy considered the role of PMS and CBM as a requirement for achieving any RCM program(MoD, 2007; [NAVSEA, 2007](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib51)). While the nature of PMS stipulates time-based approach, that of CBM relies on sensor deployments hence the place of system reliability analysis to harness the weakness in both([Cicek and Celik, 2013](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib16); [Cipollini et al., 2018](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib17); [Velasco-Gallego and Lazakis, 2022a](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib65)). In general reliability analysis tools examine the effects and risk of failure by considering quantitative and qualitative aspects of machinery [maintenance and operations](https://www.sciencedirect.com/topics/engineering/operation-and-maintenance "Learn more about maintenance and operations from ScienceDirect's AI-generated Topic Pages") data ([Karatuğ and Arslanoğlu, 2022](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib38)).

To this end, various researchers have implemented the use of tools such as [FTA](https://www.sciencedirect.com/topics/engineering/fault-tree-analysis "Learn more about FTA from ScienceDirect's AI-generated Topic Pages"), ETA and RBD mostly combined to provide maintenance analysis approach in order to overcome issues such as [discretisation](https://www.sciencedirect.com/topics/engineering/discretization "Learn more about discretisation from ScienceDirect's AI-generated Topic Pages"), linguistic restriction, and expert judgment ([Duan and Zhou, 2012](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib25); [Jun and Kim, 2017](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib34); [Kampitsis and Panagiotidou, 2022](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib37); [Khakzad et al., 2011](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib39)). Research efforts by ([Konstantinos Dikis et al., 2010](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib40); [Lazakis and Ölçer, 2015](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib41); [Lazakis et al., 2010](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib42); [Velasco-Gallego and Lazakis, 2022a](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib65)) implemented a risk and [reliability assessment methods](https://www.sciencedirect.com/topics/engineering/reliability-assessment-method "Learn more about reliability assessment methods from ScienceDirect's AI-generated Topic Pages") of FMECA and FTA as well as using Fuzzy [Multi Criteria Decision Making](https://www.sciencedirect.com/topics/engineering/multi-criteria-decision-making "Learn more about Multi Criteria Decision Making from ScienceDirect's AI-generated Topic Pages") Approach (FCDMA) in order to identify critical components and provide maintenance decision support for ships with focus on equipment risk and [criticality](https://www.sciencedirect.com/topics/engineering/criticality "Learn more about criticality from ScienceDirect's AI-generated Topic Pages") to maintenance. Other tools such as Bayesian belief networks, Monte Carlo simulation, [Markov chains](https://www.sciencedirect.com/topics/earth-and-planetary-sciences/markov-chain "Learn more about Markov chains from ScienceDirect's AI-generated Topic Pages"), [Petri Nets](https://www.sciencedirect.com/topics/earth-and-planetary-sciences/petri-net "Learn more about Petri Nets from ScienceDirect's AI-generated Topic Pages") and Weibull analysis among others have been applied to model maintenance planning ([Kabir and Papadopoulos, 2019](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib36); [Leimeister and Kolios, 2018](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib44); [Melani et al., 2018](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib48)). On the other hand, complex system reliability analysis requiring inputs that are largely non-binary and continuous with stochastic failure behaviour would require different approach to address temporal system state or a repairable [mechanical system](https://www.sciencedirect.com/topics/engineering/mechanical-systems "Learn more about mechanical system from ScienceDirect's AI-generated Topic Pages") that can operate satisfactorily at degraded condition. Recent research efforts have also focused on ship machinery real-time [anomaly detection](https://www.sciencedirect.com/topics/engineering/anomaly-detection "Learn more about anomaly detection from ScienceDirect's AI-generated Topic Pages") for fault diagnosis ([Velasco-Gallego and Lazakis, 2022a](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib65)); application of Bayesian and machine learning-based fault detection and diagnostics (Cheliotis et all, 2022); real-time data-driven missing data imputation evaluation for short-term sensor data of marine systems ([Velasco-Gallego and Lazakis, 2022a](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib65)); and the development of a time series imaging approach for fault classification ([Velasco-Gallego and Lazakis, 2022b](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib66)). Therefore, additional flexibility to produce a representative model taking all possible consideration will be required. Consequently, researchers have resorted to the use of multiple tools to accommodate system dependencies and complexities of multi system ([Marving Rausand and Arnljot, 2021](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib47); [Piadeh et al., 2018](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib54); [Velasco-Gallego and Lazakis, 2022a](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib65), [2022b](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib66)). This strategy enables the use of multiple data types for reliability analysis and the use of tools in a more flexible manner([Leimeister and Kolios, 2018](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib44)). Accordingly, a critical review highlighting the strengths and weaknesses of the reliability tools used for this research will discussed.

### 2.1. Fault tree analysis

Fault tree analysis (FTA) is a static method for analysing component faults in systems or equipment by identifying all possible causes of likely failures and impacts on the system through the logical analysis of dependencies in the basic events that lead to the undesired event, the top event of the fault tree([Lazakis et al., 2018](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib43){[NASA, 2002](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib49) #136)}. FTA is an important tool for reliability and risk analysis as it provides critical information used to prioritise the importance of the contributors to the undesired events([Relex et al., 2003](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib55)). It utilises Boolean law by applying gates and events to describe faulty components and possible event(s) that could develop a fault. Therefore, FTA is an important tool for reliability and risk analysis as it provides critical information used to prioritise the importance of the contributors to the undesired event i.e fault or failure. However, it has some shortcomings to do with sequence dependencies, temporal order of occurrence and redundancies due to standby systems, consequently DFTA was developed to overcome these constraints in the static FT.

The dynamic gates which include Priority and gate (PAND), Sequence Enforcing gate (SEQ), Functional Dependency gate (FDEP), Spare gate (SPARE) and the spare event when added to the FTA structure becomes Dynamic FTA ([NASA, 2002](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib49)). In the PAND gate events are prioritised from left to right such that the left most event (fault) is considered first before the next; similarly, SEQ considers events in left to right fashion however rather than prioritising it enforces hence ensuring that events follow the expected failure mechanism([Kabir, 2017](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib35)). On the other hand, the FDEP though evaluate events from left to right it does that considering the occurrence of primary, or causal event which is independent of other faults to the right([Kabir, 2017](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib35)). The SPARE gate and event have unique attributes and functions; though events are evaluated from left to right as obtained in other gates, the dormancy factor feature of the spare event makes lot of difference. The dormancy factor is a measure of the ratio between failure and operational rate of the spare event in the standby mode ([NASA, 2002](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib49)). A cold spare has dormancy factor 0, a hot spare has dormancy factor 1 and a warm spare has a dormancy factor between 0 and 1([Relex et al., 2003](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib55)). The application of dynamic gates and use of spare gates to analysis improvements in maintenance approach was presented in ([Lazakis et al., 2010](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib42); [Kabir, 2017](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib35) #437) both authors demonstrated how these dynamic gates can be applied to model time and sequence dependent failures.

In this regard, the dynamic gates in combination with other static gates provide a much robust yet simple structure compared to tools like Markov Chains and RBD. Therefore, DFTA is suitable for modelling complex systems failure behaviour with respect to sequence and dependencies, particularly where the temporal order of the occurrence of events is important to analysis([Chiacchio et al., 2016](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib15); [Jakkula et al., 2020](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib32); [NASA, 2002](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib49)). This is particularly important in order to account for the failure dynamics of static and dynamic system, while not disregarding the impact of environmental elements, temperature and other factors. The reliability of mechanical systems does not follow [constant failure rate](https://www.sciencedirect.com/topics/engineering/constant-failure-rate "Learn more about constant failure rate from ScienceDirect's AI-generated Topic Pages") as obtained in [electrical systems](https://www.sciencedirect.com/topics/engineering/electrical-system "Learn more about electrical systems from ScienceDirect's AI-generated Topic Pages") such as semiconductor, LED and software([Relex et al., 2003](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib55)). Reliability data bases for mechanical components such as OREDA, NUREG, NSWC, NPRD provide high quality failure rate information on various components and procedures for conducting reliability predictions ([Marving Rausand and Arnljot, 2021](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib47)). However, component failure rates for repairable mechanical systems are influenced by multiple factors and may not follow constant failure rates of generic distribution such as Weibull, Normal, [Lognormal](https://www.sciencedirect.com/topics/engineering/lognormal "Learn more about Lognormal from ScienceDirect's AI-generated Topic Pages") the likes ([Anantharaman et al., 2018](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib3); [Scheu et al., 2017](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib59)). Hence, DFTA provides a platform that is capable for analysing reparable system while considering other factors such as dependencies and temporal behaviour or partially operating state analysis([Marko Cerpin, 2002](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib46); [Zhou et al., 2022](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib68)). Therefore, this makes it very relevant in analysing system improvements as presented in ([Daya A.A 2021](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib20); [Turan et al., 2012](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib64)). Overall, these additional gates provided more scope in [DFT](https://www.sciencedirect.com/topics/earth-and-planetary-sciences/density-functional-theory "Learn more about DFT from ScienceDirect's AI-generated Topic Pages") analysis ([Kabir, 2017](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib35); [Ruijters and Stoelinga, 2015](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib56)) which can be used to factor repair or improvements due to routine maintenance. Moreover, additional outputs such as reliability importance measures and [minimal cuts sets](https://www.sciencedirect.com/topics/engineering/minimal-cut-set "Learn more about minimal cuts sets from ScienceDirect's AI-generated Topic Pages") in the DFTA are equally influenced by the logic structure of the model, meaning that the output of static FT and a dynamic FT analysis will be significantly different and reflective of the whatever dependencies exist in the model.

The DFTA provides additional outputs which are the reliability importance measures (IM) and minimal cut set. The IM provide a means to identify the most critical component/situation that contributes to the occurrence of the low/basic event leading up to equipment failure or top event occurrence ([Chen et al., 2021](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib14); [Kuzu and Senol, 2021](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib73) #768)}. Therefore assisting in identifying the event that, if improved, is most likely to produce significant improvement in equipment or system performance ([Relex et al., 2003](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib55)). In retrospect the MCS provide insights on failure or fault development, in that MCS are the smallest set of basic events, which if they all occur will result in the top event occurring ([NASA, 2002](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib49)). Therefore, improving the MCS can greatly improve overall system reliability.

Similarly, the IM could provide vital information on components to the maintenance crew and ship owners prioritization of actions that could ensure equipment/system reliability through holding of right spares onboard or additional maintenance options. The main IM approaches are Birnbaum (Bir), Fussell-Vesely(F-V) and Criticality (Cri)([Lazakis et al., 2018](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib43)). The Bir IM evaluates the occurrence of the top events based on the probability of its occurring or not occurring, hence the higher the probability the higher the chances of top event occurring([Relex et al., 2003](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib55)). Criticality (Cri) IM is calculated in a similar way to Bir IM except that it considers the probability in the occurrence of the basic event to the occurrences of the top event. The F-V calculation adopts an entirely different approach in that; it uses the minimal cut set summation i.e., the minimum number of basic events that contribute to the top event. Therefore, the F-V consider the contribution of the basic event to occurrence of the top event irrespective of how it contributes to the failure.

Maintenance planning for complex system is dynamic and therefore constantly changing, the more reason why organisation adopts different maintenance strategy that could fit the [operational requirement](https://www.sciencedirect.com/topics/engineering/operational-requirement "Learn more about operational requirement from ScienceDirect's AI-generated Topic Pages") or machinery condition ([Soliman, 2020](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib61){Heinz P. Bloch, 2006 #9)}. In this regard DFTA though a robust tool is not able to adequately address issues such as CCF, human error, natural events including subjective factors such as maintenance delays, spare part quality and skills shortages. Accordingly, to overcome some of these factors, DFTA has equally been combined with other tools to achieve additional research goals such as decision support or analysis requiring some level of subjective inputs ([Codetta-Raiteri and Portinale, 2017](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib19); [Zhou et al., 2022](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib68)). Accordingly, DFTA is combine with other tools to improve quality and coverage of analysis such as machine learning based tools for diagnostics and prognosis analysis([Cheliotis et al., 2020](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib13); [Eriksen et al., 2021](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib27)) DFTA, FMECA and other tools([Daya A.A 2021](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib20); [Karatuğ and Arslanoğlu, 2022](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib38)). On the other hand, the constraints imposed by the DFTA structure and the deterministic nature of the input as well as the output makes it restrictive to model certain machinery failures. Overcoming this challenge in this research was done through the use of FMECA, as it provides the required robust framework to holistically analyse failures with all the dynamics involved.

### 2.2. Failure mode effect and criticality analysis

Failure Mode Effect and Criticality Analysis is an evaluation technique to determine the impact of failure or malfunction of system, equipment or components failures by evaluating and prioritising the effect of individual failures ([Daya and Lazakis, 2022](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib21); [NASA, 2008](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib50)). FMECA is composed of 2 analyses, FMEA and Criticality Analysis (CA)([Fu et al., 2022](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib28)). The FMEA is focused on how equipment and system have failed or may fail to perform their function and the effects of these failures, to identify any required corrective actions for implementation to eliminate or minimize the likelihood of severity of failure. While criticality analysis is done to enable prioritization of the failure modes for potential corrective action([Astrom, 2002](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib4); [Ceylan et al., 2022](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib12)). FMECA is a widely used tool for reliability, criticality and risk analysis across industry and academia, as it does not require much technical knowledge but provides good insight into system failures or malfunction ([DoD, 1989](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib23); [Marving Rausand and Arnljot, 2021](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib47)). [Cicek and Celik (2013)](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib16) presented an approach for identifying and controlling potential failure or operational errors that trigger [crankcase](https://www.sciencedirect.com/topics/engineering/crankcases "Learn more about crankcase from ScienceDirect's AI-generated Topic Pages") explosion using FMECA. In ([Lazakis et al., 2018](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib43)) FMEA was used for defect analysis on ship main propulsion engine by identifying critical engine failures for maintenance decision making. FMEA can equally be modified for specific application as presented in ([Niculita et al., 2017](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib52); [Shafiee et al., 2016](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib60)) where a modified Ageing Failure Mode and Effects Analysis (AFMEA) and Functional Failure Mode Effects and Criticality Analysis (FFMECA) was done for the techno-economic life extension analysis of [offshore structure](https://www.sciencedirect.com/topics/engineering/offshore-structure "Learn more about offshore structure from ScienceDirect's AI-generated Topic Pages") and ship systems respectively.

FMECA is a major component used for system analysis of important maintenance concepts such as RCM and PMS as it presents a clear view of equipment, component and personnel interaction and how risk and reliability issues can be mitigated. Mechanical system component failure analysis with FMECA is generally robust particularly in establishing modes of failure and efforts to mitigate or prevent them, however is not practically possible to determine the probability of occurrence for each identified failure rate ([NSWC, 2011](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib53)). In this regard, is common to see FMECA being used alongside other tools for system reliability study ([DoD, 2005](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib24); [Melani et al., 2018](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib48)). This is more so, as the analysis depends on qualitative inputs that can be influenced by the experience or sentiments of respondents, hence subjective([Lazakis and Ölçer, 2015](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib41)). Overall, the limitation due to the subjectivity and interpretation of results can be addressed by ranking; using weights, fuzzy methods or hierarchical approach such AHP ([Lazakis and Ölçer, 2015](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib41); [Saaty, 2016](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib57)). Accordingly, this paper has adopted the use of FMECA for system reliability analysis in order to account for expert knowledge in failure and mission critical component analysis. FMECA also help capture some subjective operator sentiments which could agree or disagree with reliability results obtained from objective methodologies such as FTA([Marving Rausand and Arnljot, 2021](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib47); [NASA, 2002](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib49)). Accordingly, to address the challenge of interpretation and subjectivity in FMECA analysis a weighting method was introduced to account for experience and years in service of all respondents([Ceylan et al., 2022](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib12)). In doing these, issues of under scoring or over scoring certain failures due to inexperience or narrow judgement can be addressed, hence providing a balanced failure analysis.

### 2.3. Bayesian belief networks

Bayesian Belief Networks (BBN) provides a good platform for [dependability](https://www.sciencedirect.com/topics/engineering/dependability "Learn more about dependability from ScienceDirect's AI-generated Topic Pages") analysis, cause, effect and inferential analysis in a wide range of sectors covering health care, human reliability, machinery system reliability and decision support system ([Ahn et al., 2022](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib2); [BayesFusion, 2020](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib7); P. [Weber et al., 2012](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib67)). BBN are represented as direct acyclic graph (DAG) which consist of chance nodes (variables) representing possible outcomes of system states and a given set of arrows (connections) indicating dependability/relationships([Bahoo et al., 2022b](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib6); [BayesFusion, 2020](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib7); [Canbulat et al., 2019](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib11)). The nodes takes variable inputs in [BBN analysis](https://www.sciencedirect.com/topics/engineering/electric-network-analysis "Learn more about BBN analysis from ScienceDirect's AI-generated Topic Pages") which can be continues or discreet and are not restricted to single top event, hence providing great flexibility unlike fault tress or RBD([Kabir and Papadopoulos, 2019](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib36)). BBN can be used to represent cause and effect between parts of system or equipment by identifying potential causes of failure. Authors have used BBN for fault and diagnostic analysis as well decision support system (DSS), for instance [Jun and Kim (2017)](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib34) presented a Bayesian based fault identification system for CBM by discretising continues parameters based on [maximum likelihood estimation](https://www.sciencedirect.com/topics/engineering/maximum-likelihood-estimation "Learn more about maximum likelihood estimation from ScienceDirect's AI-generated Topic Pages") (MLE) to identify failure conditions; the research used the discretised feature as [binary inputs](https://www.sciencedirect.com/topics/engineering/binary-input "Learn more about binary inputs from ScienceDirect's AI-generated Topic Pages") for the BBN [conditional probability](https://www.sciencedirect.com/topics/engineering/conditional-probability "Learn more about conditional probability from ScienceDirect's AI-generated Topic Pages") table (CPT). Similarly to address port energy efficiency towards the reduction ships emission during port calls a strategy using BBNs was presented in ([Canbulat et al., 2019](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib11)). This research also provides how BBN conditional probability can efficiently in-cooperate expert knowledge to provide vital inputs in decision making variables in areas where there is in adequate data or literature.

[Bayesian updating](https://www.sciencedirect.com/topics/engineering/bayesian-updating "Learn more about Bayesian updating from ScienceDirect's AI-generated Topic Pages") or inference provides bases for the use of influence diagrams in decisions analysis by computing the impact of new evidence to the probability of events and the influence on all related nodes([BayesFusion, 2020](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib7)). As such BN provide a good platform for DSS especially in maintenance strategy selection when considering several dependent and independent factors. Conducting system reliability and maintenance analysis demands in puts from multiple sources which the BN platform can accommodate as compared to other tools. Papers by [Jun and Kim (2017)](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib34) and [Li et al. (2020)](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib45) provide methodologies for the use of BBN in reliability analysis, however, while ([Jun and Kim, 2017](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib34)) focused on fault diagnose ([Li et al., 2020](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib45)) emphases on [component reliability](https://www.sciencedirect.com/topics/earth-and-planetary-sciences/component-reliability "Learn more about component reliability from ScienceDirect's AI-generated Topic Pages") with limited analysis on factors affecting the reliability. Furthermore, BBN have been used to provide inferential analysis in conjunction with other tools such as Markov chain and Petri-nets especially in risk and reliability analysis ([Galagedarage Don and Khan, 2019](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib30); [Kabir and Papadopoulos, 2019](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib36); [Kampitsis and Panagiotidou, 2022](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib37); [Khakzad et al., 2011](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib39)). BBN based DSS are widely applied in maritime industry to handle operational issues such as [human factors](https://www.sciencedirect.com/topics/engineering/ergonomics "Learn more about human factors from ScienceDirect's AI-generated Topic Pages") and procedural issues such as maintenance ([Ahn and Kurt, 2020](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib1); [Kampitsis and Panagiotidou, 2022](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib37)). Similarly in the field of ship system reliability analysis ([Velasco-Gallego and Lazakis, 2022b](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib66))has presented on the use of BBN and FTA for ship machinery cooling system reliability analysis and DSS. Likewise, Bahoo et al. ([Bahoo et al., 2022](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib5)) applied a combination BBN and [Markov chain Monte Carlo simulation](https://www.sciencedirect.com/topics/engineering/markov-chain-monte-carlo-simulation "Learn more about Monte Carlo simulation Markov chains from ScienceDirect's AI-generated Topic Pages") to analyses machinery reliability estimation onboard autonomous ships to help maintenance planning and decision.

Maintenance planning and decision marking for ship systems can be complex due to the operational nature, space constraint and onboard environment. Hence maintenance as well as spare parts holding must be carefully considered so that failures and repairs are adequately prioritised to avoid problems with onboard spare parts holding and technical skills mix. Therefore, notwithstanding the rigorous efforts by authors in the field of ship system reliability there exist some important gaps in the literature. Some of this includes, not clearly identifying component criticality to system availability, maintenance action prioritization to reflect failure severity as regard vessel operational demands and selecting maintenance decision to reflect operator's sentiment. Providing solution to these problems require a systematic approach to system reliability analysis. Accordingly, this paper presents a novel methodology, which involves the combination DFTA, FMECA and BBN reliability analysis tools to conduct component criticality analysis, system failure dependency and influence as well as decision support system.

The presented methodology provides a detailed and comprehensive analysis that identifies critical components in relation to ship availability and maintenance effort in an inclusive manner that can account for operator concerns, OEMs’ recommendations, and environmental influence. Unlike data driven approaches which rely on machinery health parameters or statistical methods such as distribution, [residual life](https://www.sciencedirect.com/topics/engineering/residual-life "Learn more about residual life from ScienceDirect's AI-generated Topic Pages") estimation through [Mean Time To Failure](https://www.sciencedirect.com/topics/engineering/mean-time-to-failure "Learn more about Mean Time To Failure from ScienceDirect's AI-generated Topic Pages") (MTTF) which depend on failure rates, hence are unable to provide detailed information on failure and their causes. Other graphical approaches such as the [bathtub curve](https://www.sciencedirect.com/topics/engineering/bathtub-curve "Learn more about bathtub curve from ScienceDirect's AI-generated Topic Pages") also relies on [failure rate data](https://www.sciencedirect.com/topics/engineering/failure-rate-data "Learn more about failure rate data from ScienceDirect's AI-generated Topic Pages"), which is insufficient to identify issues such as single point failure, [common cause failures](https://www.sciencedirect.com/topics/engineering/common-cause-failure "Learn more about common cause failures from ScienceDirect's AI-generated Topic Pages") or critical components within a system and its components. Moreover, machinery failures can occur due to material or design defects, age/wear out and poor maintenance or intrusively actual maintenance action. Therefore, a hybrid approach taking into account the multiple dynamics in system reliability and failure mechanics needs to be developed. Accordingly, to develop a ship reliability analysis alongside a maintenance decision support system these selected tools provide a good match and can accommodate all the relevant variables as compared to using one or a couple of these tools. Consequently, FMECA provides outputs for mission critical components based on operator perspective, MCS provides an objective output to reflect MRO data both of which were used to build DSS using BBN. Furthermore, a case study was conducted to demonstrate the suggested novel methodology.

## 3\. Methodology

The novel methodology presents a systematic combination of quantitative and qualitative reliability analysis approaches to ship system reliability by combining DFTA, FMECA, and BBN to address some observed gaps in the literature. This includes, for example, not explicitly defining component [criticality](https://www.sciencedirect.com/topics/engineering/criticality "Learn more about criticality from ScienceDirect's AI-generated Topic Pages") to system availability or prioritising maintenance actions based on the severity of failures in relation to vessel operating needs and operator-led maintenance planning. A methodical approach to system [dependability](https://www.sciencedirect.com/topics/engineering/dependability "Learn more about dependability from ScienceDirect's AI-generated Topic Pages") analysis is necessary to provide solutions for these issues. Therefore, this research presents a unique technique for conducting component criticality analysis by utilising the particular strengths of combined reliability tools as presented in [Fig. 1](https://www.sciencedirect.com/science/article/pii/S0029801823001506#fig1).

![Fig. 1](https://ars.els-cdn.com/content/image/1-s2.0-S0029801823001506-gr1.jpg)

1.  [Download : Download high-res image (318KB)](https://ars.els-cdn.com/content/image/1-s2.0-S0029801823001506-gr1_lrg.jpg "Download high-res image (318KB)")
2.  [Download : Download full-size image](https://ars.els-cdn.com/content/image/1-s2.0-S0029801823001506-gr1.jpg "Download full-size image")

Fig. 1. Flowchart of novel methodology.

### 3.1. Subjective inputs

A key difficulty in maintenance planning is handling equipment defects that are not fully addressed in [OEM](https://www.sciencedirect.com/topics/engineering/original-equipment-manufacturer "Learn more about OEM from ScienceDirect's AI-generated Topic Pages") maintenance and troubleshooting manuals, such as environment-related faults, design restrictions, incorrect application, or unsuitable operation. These sorts of defects typically result in frequent equipment failures and [performance deterioration](https://www.sciencedirect.com/topics/engineering/performance-deterioration "Learn more about performance deterioration from ScienceDirect's AI-generated Topic Pages"), reducing system reliability and overall platform availability. However, because these defects are not well documented by the OEM and may not have been routinely experienced by operators on that or similar equipment. It is thus difficult to capture for reliability and maintenance analysis. In this context, information from operators on problems and maintenance issues would be required, thus the use of FMECA in this research.

#### 3.1.1. FMECA

FMECA is widely applied in maintenance and risk analysis to provide clear understanding and procedure on what could go wrong, how it could go wrong, why it goes wrong, and how it can be corrected or addressed ([Marving Rausand and Arnljot, 2021](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib47)). The Criticality Analysis (CA) provides a means of identifying the events, occurrence or components that need more attention to avoid more serious or catastrophic situations([Melani et al., 2018](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib48)). FMECA is a bottom-up approach which provides a [systematic methodology](https://www.sciencedirect.com/topics/engineering/systematic-methodology "Learn more about systematic methodology from ScienceDirect's AI-generated Topic Pages") to gain deep insight on failures and their course on an equipment or system. Therefore, measuring criticality in FMECA helps to explicitly bring out the most critical component failure which can assist in maintenance actions and planning. In this regard subjective operator inputs were obtained using FMECA, the relevance of which can be described in 2 folds. The first is to evaluate operator sentiments and priorities specially to do with failures and maintenance challenges such as expertise and causes of extended [down times](https://www.sciencedirect.com/topics/engineering/downtime "Learn more about down times from ScienceDirect's AI-generated Topic Pages"). This was also used to establish maintenance critical failures and machinery parts. The second aspect was to validate critical components obtained using DFTA qualitative analysis. Therefore, to establish these 2 goals using FMECA analysis a questionnaire was produced and distributed using the Qualtrics survey software; [Table 1](https://www.sciencedirect.com/science/article/pii/S0029801823001506#tbl1) is a template of the FMECA table used.

Table 1. Sample FMECA table.

| Subsystem | Component | Function | Description of Failure | Effects of Failure | Safeguards | Criticality | Severity | Likelihood | RPN |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
|  |  |  | **Mode** | **Causes** | **Detection** | **Local** | **Global** | **Influence** | **TTR** | **Prevention** | **Mitigation** | **1–10** | **1–10** | **1–10** | **CxSxL** |
|   
 |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
|   
 |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
|   
 |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |

The survey questions were aimed at identifying components that presents the greatest challenge to the conduct of maintenance onboard using [risk priority number](https://www.sciencedirect.com/topics/engineering/risk-priority-number "Learn more about risk priority number from ScienceDirect's AI-generated Topic Pages"). The RPN use 3 categorical variables namely identification, severity and likelihood usually measured in a linear scale based on increasing importance i.e 1 – 10. The scale used for the analysis is presented in [Table 2](https://www.sciencedirect.com/science/article/pii/S0029801823001506#tbl2), which shows the linear and Likert scale including colour codes representing respective scale values ([Jeong et al., 2018](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib33); [Tan et al., 2011](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib62)).

Table 2. Definition of Criteria.

![](https://ars.els-cdn.com/content/image/1-s2.0-S0029801823001506-fx1.jpg)

Furthermore, in this study, criticality was employed instead of [detectability](https://www.sciencedirect.com/topics/engineering/detectability "Learn more about detectability from ScienceDirect's AI-generated Topic Pages") since enhanced sensor and monitoring have substantially increased the level at which problems are identified, either through set alarm levels or an emergency shutdown system. For the sake of clarity, criticality, determines the immediate impact of failure event to the equipment availability and functions. Therefore, a failure mode due to which the ship will not achieve one or more of the mission's targets and/or the safety of whole vessel is at risk until the failure is rectified([NASA, 2008](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib50)). The next is Severity, severity assesses how the failure impacts on the operational availability of the equipment or system regarding normal operation and the duration it takes to be repaired or restored to normal operational levels. Severity is described as the worst potential consequence of the failure determined by the degree of injury, property damage or system damage that could occur. Lastly, likelihood and refers to the failure rate of the component including possibility and frequency of the fault occurring over a certain time frame ([DoD, 1980](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib22)).

The above explanation provides a guide to help respondents assess all the criteria against the candidate failures and components. Thereafter the responses were aggregated through a weight system to obtain single outcome to quantify the 3 criteria needed for calculating the RPN which are Criticality (C), Severity (S) and Likelihood (L). The RPN was used to get the mission criticality of the components or faults which is given by RPN = CxSxL scored on a scale 1–100; 1 being minor or low and 100 being very high score as regards impact. The FMECA was conducted through a survey completed by the engineering personnel of the organisation most of whom are either electrical or marine engineers with varied level of technical knowledge and experience. In this regard, a 2-weight system was introduced to account for experience and expertise, as presented in [Table 3](https://www.sciencedirect.com/science/article/pii/S0029801823001506#tbl3). Accordingly, all individual inputs were evaluated to reflect years of experience and specialisation of the respondents. For instance, response on piston failure by a marine engineer with 12 years’ experience will have more weight compared to that of electrical engineer with same experience and verse [vasa](https://www.sciencedirect.com/topics/engineering/vasa "Learn more about vasa from ScienceDirect's AI-generated Topic Pages") if the response were to be on [alternator](https://www.sciencedirect.com/topics/engineering/alternator "Learn more about alternator from ScienceDirect's AI-generated Topic Pages") parts.

Table 3. FMECA Respondents and weights.

| Experience | Weights | S | Weights | Ag Weight | Applied weight (%) |
| --- | --- | --- | --- | --- | --- |
| 0–5years | 50 | WKO/WKD | 0 | 50 + 0 | 0.5 |
| 5–11 years | 60 | WKDWEO/MEO | 0 | 60 + 0 | 0.6 |
| 11–15 years | 65 | WEO/MEO | 5 | 65 + 5 | 0.7 |
| 15–20 years | 70 | FSWEO/FSMEO | 10 | 70 + 10 | 0.8 |
| 20–24 years | 75 | FSMO/FSG CMDR | 15 | 75 + 15 | 0.9 |
| 24–28 years | 80 | FSMO/FSG CMDR | 20 | 80 + 20 | 1 |
| 28–35 years | 100 | FSG CMDR | 0 | 100 + 0 | 1 |

Adopting the above weights, individual responses were evaluated according to experience and specialisation to obtain the population mean, equation [(1)](https://www.sciencedirect.com/science/article/pii/S0029801823001506#fd1). Thereafter a weighted average is taken for each grouped experience, equation [(2)](https://www.sciencedirect.com/science/article/pii/S0029801823001506#fd2), which provides single category for criticality analysis to obtain the RPN using equation [(3)](https://www.sciencedirect.com/science/article/pii/S0029801823001506#fd3). However, the linear values used for the criteria were between |1–10| while the FMAEC RPN was 0 ≤RPN ≥300. In this regard equation [(4)](https://www.sciencedirect.com/science/article/pii/S0029801823001506#fd4) was used to obtain normalised RPN within the range of |1–100|equation 1$<math><mrow is="true"><mi is="true">p</mi><mi is="true">o</mi><mi is="true">p</mi><mi is="true">u</mi><mi is="true">l</mi><mi is="true">a</mi><mi is="true">t</mi><mi is="true">i</mi><mi is="true">o</mi><mi is="true">n</mi><mspace width="0.25em" is="true"></mspace><mi is="true">m</mi><mi is="true">e</mi><mi is="true">a</mi><mi is="true">n</mi><mspace width="0.25em" is="true"></mspace><mi is="true">μ</mi><mo linebreak="badbreak" is="true">=</mo><mfrac is="true"><mrow is="true"><mo is="true">∑</mo><mi is="true">x</mi></mrow><mi is="true">N</mi></mfrac></mrow></math>$equation 2$<math><mrow is="true"><mi is="true">W</mi><mi is="true">e</mi><mi is="true">i</mi><mi is="true">g</mi><mi is="true">h</mi><mi is="true">t</mi><mi is="true">e</mi><mi is="true">d</mi><mspace width="0.25em" is="true"></mspace><mi is="true">a</mi><mi is="true">v</mi><mi is="true">e</mi><mi is="true">r</mi><mi is="true">a</mi><mi is="true">g</mi><mi is="true">e</mi><mspace width="0.25em" is="true"></mspace><mi is="true">w</mi><mo linebreak="badbreak" is="true">=</mo><mfrac is="true"><mrow is="true"><munderover is="true"><mo is="true">∑</mo><mrow is="true"><mi is="true">i</mi><mo linebreak="badbreak" is="true">=</mo><mn is="true">1</mn></mrow><mi is="true">n</mi></munderover><mrow is="true"><msub is="true"><mi is="true">w</mi><mi is="true">i</mi></msub><msub is="true"><mi is="true">X</mi><mi is="true">i</mi></msub></mrow></mrow><mrow is="true"><munderover is="true"><mo is="true">∑</mo><mrow is="true"><mi is="true">i</mi><mo linebreak="badbreak" is="true">=</mo><mn is="true">1</mn></mrow><mi is="true">n</mi></munderover><msub is="true"><mi is="true">w</mi><mi is="true">i</mi></msub></mrow></mfrac></mrow></math>$equation 3$<math><mrow is="true"><mi is="true">R</mi><mi is="true">P</mi><mi is="true">N</mi><mo linebreak="badbreak" is="true">=</mo><mrow is="true"><munder is="true"><mo is="true">∑</mo><mrow is="true"><mi is="true">i</mi><mo linebreak="badbreak" is="true">=</mo><mo linebreak="badbreak" is="true">≤</mo><mn is="true">1</mn></mrow></munder><mrow is="true"><mi is="true">C</mi><msub is="true"><mi is="true">w</mi><mi is="true">i</mi></msub><mo is="true">×</mo><msub is="true"><mrow is="true"><mi is="true">S</mi><mi is="true">w</mi></mrow><mi is="true">i</mi></msub></mrow></mrow><mo linebreak="goodbreak" is="true">×</mo><mi is="true">L</mi><msub is="true"><mi is="true">w</mi><mi is="true">i</mi></msub></mrow></math>$equation 4$<math><mrow is="true"><msub is="true"><mrow is="true"><mi is="true">R</mi><mi is="true">P</mi><mi is="true">N</mi></mrow><mrow is="true"><mi is="true">n</mi><mi is="true">o</mi><mi is="true">r</mi><mi is="true">m</mi></mrow></msub><mo linebreak="badbreak" is="true">=</mo><mfrac is="true"><mrow is="true"><mi is="true">X</mi><mo linebreak="badbreak" is="true">−</mo><mi mathvariant="italic" is="true">min</mi><mrow is="true"><mo stretchy="true" is="true">(</mo><mi is="true">X</mi><mo stretchy="true" is="true">)</mo></mrow></mrow><mrow is="true"><mi mathvariant="italic" is="true">max</mi><mrow is="true"><mo stretchy="true" is="true">(</mo><mi is="true">X</mi><mo stretchy="true" is="true">)</mo></mrow><mo linebreak="badbreak" is="true">−</mo><mi mathvariant="italic" is="true">min</mi><mrow is="true"><mo stretchy="true" is="true">(</mo><mi is="true">X</mi><mo stretchy="true" is="true">)</mo></mrow></mrow></mfrac><mo linebreak="goodbreak" is="true">=</mo></mrow></math>$

### 3.2. Objective inputs

The methodology adopted in this research draws from multiple data sources some of which are raw machinery log data, [maintenance and repair](https://www.sciencedirect.com/topics/earth-and-planetary-sciences/repair-and-maintenance "Learn more about maintenance and repair from ScienceDirect's AI-generated Topic Pages") data including technical reports, others are output from tools used in the research. This process enables a more robust analysis especially considering that the duration of the research will not allow verification or implementation of the methodology onboard. Accordingly, the objective inputs are independent numerical variables and not controlled by the modeler. These includes failure rates obtained from machinery failure data used as inputs for the DFTA, RPN values from FMECA, and MCS probabilities from the DFTA results used as inputs in the BBN. Furthermore, availability percentages from the BBN were used to build the DSS model which was complemented by MCS from the DFTA.

#### 3.2.1. DFTA

The dynamic [fault tree analysis](https://www.sciencedirect.com/topics/engineering/fault-tree-analysis "Learn more about fault tree analysis from ScienceDirect's AI-generated Topic Pages") is an extension of standard fault tree analysis that provides for time or sequence dependent analysis and can also prioritise events for analysis. DFTA is selected for this study in order to utilise its system dependent relationship on the effect of component failures. The DFTA tool used for machinery/system reliability and availability analysis used input data generated from the operational records of 4 [diesel](https://www.sciencedirect.com/topics/earth-and-planetary-sciences/diesel "Learn more about diesel from ScienceDirect's AI-generated Topic Pages") generators for a ship [power generation system](https://www.sciencedirect.com/topics/engineering/power-generation-system "Learn more about power generation system from ScienceDirect's AI-generated Topic Pages"). Therefore, a DFTA structure representing the functional ship power generation system as well as the individual diesel generators was built. System reliability in DFTA involves generating a qualitative model of the fault tree usually from the [minimal cut sets](https://www.sciencedirect.com/topics/engineering/minimal-cut-set "Learn more about minimal cut sets from ScienceDirect's AI-generated Topic Pages") on the [logic gate](https://www.sciencedirect.com/topics/engineering/logic-gate "Learn more about logic gate from ScienceDirect's AI-generated Topic Pages") of the fault tree. Thereafter, [quantitative analysis](https://www.sciencedirect.com/topics/engineering/quantitative-measurement "Learn more about quantitative analysis from ScienceDirect's AI-generated Topic Pages") using reliability and [maintainability](https://www.sciencedirect.com/topics/engineering/maintainability "Learn more about maintainability from ScienceDirect's AI-generated Topic Pages") data such as failure rates/frequency, failure probability, [mean time to failure](https://www.sciencedirect.com/topics/engineering/mean-time-to-failure "Learn more about mean time to failure from ScienceDirect's AI-generated Topic Pages") or repair rate can be used ([Relex et al., 2003](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib55)), by calculating the unavailability and the unreliability of the system to be done.

Accordingly, failure and maintenance data over a period of 6 calendar years obtained from the maintenance records was processed to generate components failure rates (⋌) based on equation [(5)](https://www.sciencedirect.com/science/article/pii/S0029801823001506#fd5). The model structure was built using both static and dynamic FT gates and events to reflect the mode of failures and in other cases dependency and sequence. Therefore, top events and sub-events were modelled using dynamic gates while gates connecting to the main system were modelled using static FTs this procedure is necessary to reduce memory usage and improve calculation time. The probabilities for the static gates used were generally AND gate equation [(6)](https://www.sciencedirect.com/science/article/pii/S0029801823001506#fd6), OR gate equation [(7)](https://www.sciencedirect.com/science/article/pii/S0029801823001506#fd7) and Voting gate . Voting gate (equation [(8)](https://www.sciencedirect.com/science/article/pii/S0029801823001506#fd8)) account for multiple connected components (_k out of n_) such as injection nozzles, cylinder blocks, fuel day tanks or supply line, this is because correct functioning of system requires all component but is not necessarily impaired due to a few faulty ones.equation 5$<math><mrow is="true"><mo is="true">⋌</mo><mo linebreak="badbreak" is="true">=</mo><mfrac is="true"><mi is="true">n</mi><mi is="true">τ</mi></mfrac></mrow></math>$Where n is number of failures (10<sup>6</sup>) and $<math><mrow is="true"><mi is="true">τ</mi></mrow></math>$ is aggregated time in service of individual DG. The inputs for the gates are obtained with the below equations.

Probability of occurrence of an AND gate = equation 6$<math><mrow is="true"><mi is="true">Pr</mi><mrow is="true"><mo stretchy="true" is="true">{</mo><mi is="true">A</mi><mo stretchy="true" is="true">}</mo></mrow><mo linebreak="badbreak" is="true">=</mo><mi mathvariant="italic" is="true">Pr</mi><mrow is="true"><mo stretchy="true" is="true">{</mo><msub is="true"><mi is="true">A</mi><mn is="true">1</mn></msub><mo stretchy="true" is="true">}</mo></mrow><mo linebreak="badbreak" is="true">•</mo><mi mathvariant="italic" is="true">Pr</mi><mrow is="true"><mo stretchy="true" is="true">{</mo><mrow is="true"><msub is="true"><mi is="true">A</mi><mn is="true">2</mn></msub><mo stretchy="true" is="true">|</mo><msub is="true"><mi is="true">A</mi><mn is="true">1</mn></msub></mrow><mo stretchy="true" is="true">}</mo></mrow><mo linebreak="badbreak" is="true">•</mo><mo is="true">…</mo><mo linebreak="goodbreak" is="true">•</mo><mi mathvariant="italic" is="true">Pr</mi><mrow is="true"><mo stretchy="true" is="true">{</mo><mrow is="true"><msub is="true"><mi is="true">A</mi><mi is="true">n</mi></msub><mo stretchy="true" is="true">|</mo><msub is="true"><mi is="true">A</mi><mrow is="true"><mn is="true">1</mn><mo is="true">,</mo></mrow></msub><msub is="true"><mi is="true">A</mi><mrow is="true"><mn is="true">2</mn><mo is="true">,</mo></mrow></msub><mo is="true">…</mo><mo is="true">,</mo><msub is="true"><mi is="true">A</mi><mrow is="true"><mi is="true">n</mi><mo linebreak="badbreak" is="true">−</mo><mn is="true">1</mn></mrow></msub></mrow><mo stretchy="true" is="true">}</mo></mrow></mrow></math>$

If all events are independent, then$<math><mrow is="true"><mi is="true">Pr</mi><mrow is="true"><mo stretchy="true" is="true">{</mo><mi is="true">A</mi><mo stretchy="true" is="true">}</mo></mrow><mo linebreak="badbreak" is="true">=</mo><mi mathvariant="italic" is="true">Pr</mi><mrow is="true"><mo stretchy="true" is="true">{</mo><msub is="true"><mi is="true">A</mi><mn is="true">1</mn></msub><mo stretchy="true" is="true">}</mo></mrow><mo linebreak="badbreak" is="true">•</mo><mi mathvariant="italic" is="true">Pr</mi><mrow is="true"><mo stretchy="true" is="true">{</mo><msub is="true"><mi is="true">A</mi><mn is="true">2</mn></msub><mo stretchy="true" is="true">}</mo></mrow><mo linebreak="badbreak" is="true">•</mo><mo is="true">…</mo><mo linebreak="goodbreak" is="true">•</mo><mi mathvariant="italic" is="true">Pr</mi><mrow is="true"><mo stretchy="true" is="true">{</mo><msub is="true"><mi is="true">A</mi><mi is="true">n</mi></msub><mo stretchy="true" is="true">}</mo></mrow></mrow></math>$

For an OR gate given A<sub>1,</sub>A<sub>2</sub>….., A<sub>n</sub> as inputs and A is the output of the OR gate, the probability of its occurrence (top event) = equation 7$<math><mrow is="true"><mi mathvariant="italic" is="true">Pr</mi><mrow is="true"><mo stretchy="true" is="true">{</mo><mi is="true">A</mi><mo stretchy="true" is="true">}</mo></mrow><mo linebreak="badbreak" is="true">=</mo><mi mathvariant="italic" is="true">Pr</mi><mrow is="true"><mo stretchy="true" is="true">{</mo><msub is="true"><mi is="true">A</mi><mn is="true">1</mn></msub><mo stretchy="true" is="true">}</mo></mrow><mo linebreak="badbreak" is="true">+</mo><mi mathvariant="italic" is="true">Pr</mi><mrow is="true"><mo stretchy="true" is="true">{</mo><mrow is="true"><msub is="true"><mi is="true">A</mi><mn is="true">2</mn></msub><mo stretchy="true" is="true">|</mo><msub is="true"><mrow is="true"><mo is="true">∼</mo><mi is="true">A</mi></mrow><mn is="true">1</mn></msub></mrow><mo stretchy="true" is="true">}</mo></mrow><mo linebreak="badbreak" is="true">+</mo><mo is="true">…</mo><mo linebreak="goodbreak" is="true">+</mo><mi mathvariant="italic" is="true">Pr</mi><mrow is="true"><mo stretchy="true" is="true">{</mo><mrow is="true"><msub is="true"><mi is="true">A</mi><mi is="true">n</mi></msub><mo stretchy="true" is="true">|</mo><msub is="true"><mrow is="true"><mo is="true">∼</mo><mi is="true">A</mi></mrow><mn is="true">1</mn></msub><mo is="true">,</mo><msub is="true"><mrow is="true"><mo is="true">∼</mo><mi is="true">A</mi></mrow><mn is="true">2</mn></msub><mo is="true">,</mo><mo is="true">…</mo><msub is="true"><mrow is="true"><mo is="true">∼</mo><mi is="true">A</mi></mrow><mrow is="true"><mi is="true">n</mi><mo linebreak="badbreak" is="true">−</mo><mn is="true">1</mn></mrow></msub></mrow><mo stretchy="true" is="true">}</mo></mrow></mrow></math>$

If all events are independent$<math><mrow is="true"><mi mathvariant="italic" is="true">Pr</mi><mrow is="true"><mo stretchy="true" is="true">{</mo><mi is="true">A</mi><mo stretchy="true" is="true">}</mo></mrow><mo linebreak="badbreak" is="true">=</mo><mi mathvariant="italic" is="true">Pr</mi><mrow is="true"><mo stretchy="true" is="true">{</mo><msub is="true"><mi is="true">A</mi><mn is="true">1</mn></msub><mo stretchy="true" is="true">}</mo></mrow><mo linebreak="badbreak" is="true">+</mo><mi mathvariant="italic" is="true">Pr</mi><mrow is="true"><mo stretchy="true" is="true">{</mo><msub is="true"><mi is="true">A</mi><mn is="true">2</mn></msub><mo stretchy="true" is="true">}</mo></mrow><mspace width="0.25em" is="true"></mspace><mo linebreak="badbreak" is="true">•</mo><mi mathvariant="italic" is="true">Pr</mi><mrow is="true"><mo stretchy="true" is="true">{</mo><msub is="true"><mrow is="true"><mo is="true">∼</mo><mi is="true">A</mi></mrow><mn is="true">1</mn></msub><mo stretchy="true" is="true">}</mo></mrow><mo linebreak="badbreak" is="true">+</mo><mo is="true">…</mo><mo linebreak="badbreak" is="true">+</mo><mi mathvariant="italic" is="true">Pr</mi><mrow is="true"><mo stretchy="true" is="true">{</mo><msub is="true"><mi is="true">A</mi><mi is="true">n</mi></msub><mo stretchy="true" is="true">}</mo></mrow><mo linebreak="badbreak" is="true">•</mo><mi mathvariant="italic" is="true">Pr</mi><mrow is="true"><mo stretchy="true" is="true">{</mo><msub is="true"><mrow is="true"><mo is="true">∼</mo><mi is="true">A</mi></mrow><mn is="true">1</mn></msub><mo stretchy="true" is="true">}</mo></mrow><mspace width="0.25em" is="true"></mspace><mo linebreak="badbreak" is="true">•</mo><mi mathvariant="italic" is="true">Pr</mi><mrow is="true"><mo stretchy="true" is="true">{</mo><msub is="true"><mrow is="true"><mo is="true">∼</mo><mi is="true">A</mi></mrow><mn is="true">2</mn></msub><mo stretchy="true" is="true">}</mo></mrow><mo linebreak="badbreak" is="true">•</mo><mo is="true">…</mo><mo linebreak="goodbreak" is="true">•</mo><mi mathvariant="italic" is="true">Pr</mi><mrow is="true"><mo stretchy="true" is="true">{</mo><msub is="true"><mrow is="true"><mo is="true">∼</mo><mi is="true">A</mi></mrow><mrow is="true"><mi is="true">n</mi><mo linebreak="badbreak" is="true">−</mo><mn is="true">1</mn></mrow></msub><mo stretchy="true" is="true">}</mo></mrow></mrow></math>$$<math><mrow is="true"><mo is="true">=</mo><mi is="true">P</mi><mi is="true">r</mi><mi is="true">P</mi><mrow is="true"><mo stretchy="true" is="true">{</mo><msub is="true"><mi is="true">A</mi><mn is="true">1</mn></msub><mo stretchy="true" is="true">}</mo></mrow><mo linebreak="badbreak" is="true">+</mo><mi mathvariant="italic" is="true">Pr</mi><mrow is="true"><mo stretchy="true" is="true">{</mo><msub is="true"><mi is="true">A</mi><mn is="true">2</mn></msub><mo stretchy="true" is="true">}</mo></mrow><mspace width="0.25em" is="true"></mspace><mo linebreak="badbreak" is="true">•</mo><mrow is="true"><mo stretchy="true" is="true">(</mo><mrow is="true"><mn is="true">1</mn><mo linebreak="badbreak" is="true">−</mo><mi mathvariant="italic" is="true">Pr</mi><mrow is="true"><mo stretchy="true" is="true">{</mo><msub is="true"><mi is="true">A</mi><mn is="true">1</mn></msub><mo stretchy="true" is="true">}</mo></mrow></mrow><mo stretchy="true" is="true">)</mo></mrow><mo linebreak="badbreak" is="true">+</mo><mo is="true">…</mo><mo linebreak="badbreak" is="true">+</mo><mi mathvariant="italic" is="true">Pr</mi><mrow is="true"><mo stretchy="true" is="true">{</mo><msub is="true"><mi is="true">A</mi><mi is="true">n</mi></msub><mo stretchy="true" is="true">}</mo></mrow><mo linebreak="badbreak" is="true">•</mo><mrow is="true"><mo stretchy="true" is="true">(</mo><mrow is="true"><mn is="true">1</mn><mo linebreak="badbreak" is="true">−</mo><mi mathvariant="italic" is="true">Pr</mi><mrow is="true"><mo stretchy="true" is="true">{</mo><msub is="true"><mi is="true">A</mi><mn is="true">1</mn></msub><mo stretchy="true" is="true">}</mo></mrow></mrow><mo stretchy="true" is="true">)</mo></mrow><mo linebreak="badbreak" is="true">•</mo><mrow is="true"><mo stretchy="true" is="true">(</mo><mrow is="true"><mn is="true">1</mn><mo linebreak="badbreak" is="true">−</mo><mi mathvariant="italic" is="true">Pr</mi><mrow is="true"><mo stretchy="true" is="true">{</mo><msub is="true"><mi is="true">A</mi><mn is="true">2</mn></msub><mo stretchy="true" is="true">}</mo></mrow></mrow><mo stretchy="true" is="true">)</mo></mrow><mo linebreak="badbreak" is="true">•</mo><mo is="true">…</mo><mo linebreak="goodbreak" is="true">•</mo><mrow is="true"><mo stretchy="true" is="true">(</mo><mrow is="true"><mn is="true">1</mn><mo linebreak="badbreak" is="true">−</mo><mi mathvariant="italic" is="true">Pr</mi><mrow is="true"><mo stretchy="true" is="true">{</mo><msub is="true"><mi is="true">A</mi><mrow is="true"><mi is="true">n</mi><mo linebreak="badbreak" is="true">−</mo><mn is="true">1</mn></mrow></msub><mo stretchy="true" is="true">}</mo></mrow></mrow><mo stretchy="true" is="true">)</mo></mrow></mrow></math>$$<math><mrow is="true"><mo is="true">=</mo><mn is="true">1</mn><mo linebreak="badbreak" is="true">−</mo><mrow is="true"><mo stretchy="true" is="true">(</mo><mrow is="true"><mn is="true">1</mn><mo linebreak="badbreak" is="true">−</mo><mi mathvariant="italic" is="true">Pr</mi><mrow is="true"><mo stretchy="true" is="true">{</mo><msub is="true"><mi is="true">A</mi><mn is="true">1</mn></msub><mo stretchy="true" is="true">}</mo></mrow></mrow><mo stretchy="true" is="true">)</mo></mrow><mo linebreak="badbreak" is="true">•</mo><mrow is="true"><mo stretchy="true" is="true">(</mo><mrow is="true"><mn is="true">1</mn><mo linebreak="badbreak" is="true">−</mo><mi mathvariant="italic" is="true">Pr</mi><mrow is="true"><mo stretchy="true" is="true">{</mo><msub is="true"><mi is="true">A</mi><mn is="true">2</mn></msub><mo stretchy="true" is="true">}</mo></mrow></mrow><mo stretchy="true" is="true">)</mo></mrow><mo linebreak="badbreak" is="true">•</mo><mo is="true">…</mo><mo linebreak="goodbreak" is="true">•</mo><mrow is="true"><mo stretchy="true" is="true">(</mo><mrow is="true"><mn is="true">1</mn><mo linebreak="badbreak" is="true">−</mo><mi mathvariant="italic" is="true">Pr</mi><mrow is="true"><mo stretchy="true" is="true">{</mo><msub is="true"><mi is="true">A</mi><mrow is="true"><mi is="true">n</mi><mo linebreak="badbreak" is="true">−</mo><mn is="true">1</mn></mrow></msub><mo stretchy="true" is="true">}</mo></mrow></mrow><mo stretchy="true" is="true">)</mo></mrow></mrow></math>$

In the above formular A is the top event, A<sub>1</sub>, A<sub>2</sub> …., A<sub>n</sub> are lower events.

Voting gate:equation 8$<math><mrow is="true"><mi is="true">P</mi><mi is="true">r</mi><mi is="true">A</mi><mo linebreak="badbreak" is="true">=</mo><msubsup is="true"><mi is="true">C</mi><mi is="true">k</mi><mi is="true">n</mi></msubsup><msup is="true"><mrow is="true"><mo stretchy="true" is="true">(</mo><mi is="true">r</mi><mo stretchy="true" is="true">)</mo></mrow><mi is="true">k</mi></msup><msup is="true"><mrow is="true"><mo stretchy="true" is="true">(</mo><mrow is="true"><mn is="true">1</mn><mo linebreak="badbreak" is="true">−</mo><mi is="true">r</mi></mrow><mo stretchy="true" is="true">)</mo></mrow><mrow is="true"><mi is="true">n</mi><mo linebreak="badbreak" is="true">−</mo><mi is="true">k</mi></mrow></msup><mo linebreak="badbreak" is="true">+</mo><mo is="true">…</mo><mo linebreak="goodbreak" is="true">+</mo><msubsup is="true"><mi is="true">C</mi><mi is="true">n</mi><mi is="true">n</mi></msubsup><mspace width="0.25em" is="true"></mspace><msup is="true"><mrow is="true"><mo stretchy="true" is="true">(</mo><mi is="true">r</mi><mo stretchy="true" is="true">)</mo></mrow><mi is="true">n</mi></msup><msup is="true"><mrow is="true"><mo stretchy="true" is="true">(</mo><mrow is="true"><mn is="true">1</mn><mo linebreak="badbreak" is="true">−</mo><mi is="true">r</mi></mrow><mo stretchy="true" is="true">)</mo></mrow><mrow is="true"><mi is="true">n</mi><mo linebreak="badbreak" is="true">−</mo><mi is="true">n</mi></mrow></msup></mrow></math>$

The MCS for top event is obtain via equation [(9)](https://www.sciencedirect.com/science/article/pii/S0029801823001506#fd9).equation 9$<math><mrow is="true"><mi is="true">T</mi><mo linebreak="badbreak" is="true">=</mo><msub is="true"><mi is="true">M</mi><mn is="true">1</mn></msub><mo linebreak="goodbreak" is="true">+</mo><msub is="true"><mi is="true">M</mi><mn is="true">2</mn></msub><mo linebreak="goodbreak" is="true">+</mo><mo is="true">…</mo><mo linebreak="goodbreak" is="true">+</mo><msub is="true"><mi is="true">M</mi><mi is="true">K</mi></msub></mrow></math>$Where T is the top event and M<sub>i</sub> are the MSC. On the other hand, the MCS for a specific component can be given by $<math><mrow is="true"><msub is="true"><mi is="true">M</mi><mi is="true">i</mi></msub><mo linebreak="goodbreak" linebreakstyle="after" is="true">=</mo><msub is="true"><mi is="true">X</mi><mn is="true">1</mn></msub><mo linebreak="goodbreak" linebreakstyle="after" is="true">•</mo><msub is="true"><mi is="true">X</mi><mn is="true">2</mn></msub><mo linebreak="goodbreak" linebreakstyle="after" is="true">•</mo><mo is="true">…</mo><mo linebreak="goodbreak" linebreakstyle="after" is="true">•</mo><msub is="true"><mi is="true">X</mi><mi is="true">n</mi></msub></mrow></math>$ equation 10

The MCS from the DFTA was used as input for the BBN condition probability analysis.

### 3.3. BBN

Bayesian belief networks provide efficient and flexible platform for the conduct of numerical analysis to aid decision making impacted by conflicting priorities. BBNs can be up dated with new data at any point during the analysis thereby providing a very efficient tool for decision support system especially for complex system maintenance analysis([Sakar et al., 2021](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib58)). [BBN analysis](https://www.sciencedirect.com/topics/engineering/electric-network-analysis "Learn more about BBN analysis from ScienceDirect's AI-generated Topic Pages") is conducted based on DAG structure consisting of nodes of various shapes representing events and their probabilities, connected to arrows indicating dependencies or influence. [Conditional probability](https://www.sciencedirect.com/topics/engineering/conditional-probability "Learn more about Conditional probability from ScienceDirect's AI-generated Topic Pages") tables (CPT) of discrete or continues variables provide inputs for the nodes in the influence diagrams. The CPT can be updated according to data availability which provide the evidence (E) and event occurring. The evidence is used by the BN's inference engine to update the prior occurrence of event, equation [(13)](https://www.sciencedirect.com/science/article/pii/S0029801823001506#fd13) ([F.V. Jensen, 2007](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib29)).equation 13$<math><mrow is="true"><mi is="true">P</mi><mrow is="true"><mo stretchy="true" is="true">(</mo><mrow is="true"><mi is="true">U</mi><mo stretchy="true" is="true">|</mo><mi is="true">E</mi></mrow><mo stretchy="true" is="true">)</mo></mrow><mo linebreak="badbreak" is="true">=</mo><mfrac is="true"><mrow is="true"><mi is="true">P</mi><mrow is="true"><mo stretchy="true" is="true">(</mo><mrow is="true"><mi is="true">U</mi><mo is="true">,</mo><mi is="true">E</mi></mrow><mo stretchy="true" is="true">)</mo></mrow></mrow><mrow is="true"><mi is="true">P</mi><mrow is="true"><mo stretchy="true" is="true">(</mo><mi is="true">E</mi><mo stretchy="true" is="true">)</mo></mrow></mrow></mfrac><mo linebreak="badbreak" is="true">=</mo><mfrac is="true"><mrow is="true"><mi is="true">P</mi><mrow is="true"><mo stretchy="true" is="true">(</mo><mrow is="true"><mi is="true">U</mi><mo is="true">,</mo><mi is="true">E</mi></mrow><mo stretchy="true" is="true">)</mo></mrow></mrow><mrow is="true"><msub is="true"><mo is="true">∑</mo><mi is="true">U</mi></msub><mi is="true">P</mi><mrow is="true"><mo stretchy="true" is="true">(</mo><mrow is="true"><mi is="true">U</mi><mo is="true">,</mo><mi is="true">E</mi></mrow><mo stretchy="true" is="true">)</mo></mrow></mrow></mfrac></mrow></math>$

The above equation represents the overall structure of the influence diagram for a BN structure analysis. In this case the conditional probabilities of failure are presented as parent event and faults are presented as children P (Failure│Fault event). In this regard the influence diagrams for the building the maintenance DSS was generated using the CPT output of BBN which provides the availability of the DGs. Overall, the parent/child relationship of the BN structure is derived from the Bayesian theorem and chain rule that enables the quantification of relationships among the variables. Hence the [joint probability distribution](https://www.sciencedirect.com/topics/engineering/joint-probability-distribution "Learn more about joint probability distribution from ScienceDirect's AI-generated Topic Pages") _of P(U)_ represented by child(ren) _A_<sub><em>i</em></sub> for each node on the network can be evaluated based on equation [(14)](https://www.sciencedirect.com/science/article/pii/S0029801823001506#fd14).equation 14$<math><mrow is="true"><mi is="true">P</mi><mrow is="true"><mo stretchy="true" is="true">(</mo><mi is="true">U</mi><mo stretchy="true" is="true">)</mo></mrow><mo linebreak="badbreak" is="true">=</mo><mrow is="true"><munderover is="true"><mo is="true">∏</mo><mrow is="true"><mi is="true">i</mi><mo linebreak="badbreak" is="true">=</mo><mn is="true">1</mn></mrow><mi is="true">n</mi></munderover><mrow is="true"><mi is="true">P</mi><mrow is="true"><mo stretchy="true" is="true">(</mo><mrow is="true"><msub is="true"><mi is="true">A</mi><mi is="true">i</mi></msub><mo stretchy="true" is="true">|</mo><mi is="true">P</mi><mi is="true">a</mi><mrow is="true"><mo stretchy="true" is="true">(</mo><msub is="true"><mi is="true">A</mi><mi is="true">i</mi></msub><mo stretchy="true" is="true">)</mo></mrow></mrow><mo stretchy="true" is="true">)</mo></mrow></mrow></mrow></mrow></math>$Where _Pa(A_<sub><em>i</em></sub>_)_ are the parents of _Ai in P(U)_ reflects the overall relation of the nodes in the network.

Accordingly, the Bayesian network and influence diagram for the DSS were build using the Genie software ([BayesFusion, 2020](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib7)). Building the DSS require different approach as it requires utility inputs as value for decision choices. Therefore, first step in BBN analysis was to get sub-system availability using the MCS probability of occurrence obtained from the [DFT](https://www.sciencedirect.com/topics/earth-and-planetary-sciences/density-functional-theory "Learn more about DFT from ScienceDirect's AI-generated Topic Pages") analysis used as probabilities for the CPT of all the chance nodes. The BN chance nodes have 3 levels, first level identifies the probability of occurrence of the fault as a child of a component failure indicating either failed or not failed. The failure node represent components that are linked as child nodes to subsystem node which provides the output as either available or not available depending on probability of occurrence of MCS in CPT; [Fig. 2](https://www.sciencedirect.com/science/article/pii/S0029801823001506#fig2) presents a simple sketch of the BBN structure node relationships and options.

![Fig. 2](https://ars.els-cdn.com/content/image/1-s2.0-S0029801823001506-gr2.jpg)

1.  [Download : Download high-res image (263KB)](https://ars.els-cdn.com/content/image/1-s2.0-S0029801823001506-gr2_lrg.jpg "Download high-res image (263KB)")
2.  [Download : Download full-size image](https://ars.els-cdn.com/content/image/1-s2.0-S0029801823001506-gr2.jpg "Download full-size image")

Fig. 2. BBN structure node relationships and options.

The BN availability output together with the FMECA RPN provides vital inputs for the DSS in addition to maintenance strategy choices. The influence diagram for the maintenance decision support use additional nodes namely, decision and utility (value) nodes, each of which provides a complementary evaluation of the input variables. The decision nodes take in variable assigned by the [decision maker](https://www.sciencedirect.com/topics/engineering/decision-maker "Learn more about decision maker from ScienceDirect's AI-generated Topic Pages") in order to model available decision variables ([BayesFusion, 2020](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib7)). In this case the decision variables are the maintenance strategy options in [Table 4](https://www.sciencedirect.com/science/article/pii/S0029801823001506#tbl4).The second node is the value node also called utility, this node provides a measure of the [desirability](https://www.sciencedirect.com/topics/engineering/desirability "Learn more about desirability from ScienceDirect's AI-generated Topic Pages") of the output of the decision process and quantified by the utility of each possible outcome of the [parent node](https://www.sciencedirect.com/topics/engineering/parent-node "Learn more about parent node from ScienceDirect's AI-generated Topic Pages") ([BayesFusion, 2020](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib7)). The last node is the chance nodes which contain [random variables](https://www.sciencedirect.com/topics/engineering/random-variable-xi "Learn more about random variables from ScienceDirect's AI-generated Topic Pages") representing uncertainties or probabilities that are relevant to the occurrence of the events([BayesFusion, 2020](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib7)). As explained earlier, they represent probabilities of MCS of the components of each sub-system as inputs in CPT. Therefore, these 3 nodes formed the methodology of the DSS which interpret the desired outcomes based on the available choices while; the value node takes in continues variables as a measure of the parent nodes (subcomponent) criticality. In this way the utility value nodes provide the expected utility of a parent node or top event feeding it to decision node to get its availability percentage; while the decision node in the influence diagram contains maintenance decision choices which are dependent on the RPN variables inputs in value nodes as shown [Table 5](https://www.sciencedirect.com/science/article/pii/S0029801823001506#tbl5).

Table 4. Maintenance strategy options.

| Maintenance Strategy | Definition | RPN Range |
| --- | --- | --- |
| Corrective Action | This is recommended for very high to high mission critical component or faults for example sea water supply pump impeller, fuel supply pump, automatic voltage regulator faults etc. | 75–100 |
| Condition Monitoring | This strategy serves as intervention to ensure system availability targeted at component or failures whose early identification could avert major operational delays. | 55–75 |
| Planned Maintenance System | The PMS maintenance choices prioritise time dependent component failures with no immediate impacts to availability repair requirements. | 35–55 |
| Delay Action | Delay action maintenance choice is directed at those components with good resilience or sufficient redundancy such that there is little or no danger to personnel and system safety. | 0–35 |

Table 5. DSS ranking scale.

![](https://ars.els-cdn.com/content/image/1-s2.0-S0029801823001506-fx2.jpg)

The definition in [Table 4](https://www.sciencedirect.com/science/article/pii/S0029801823001506#tbl4) provide a general guidance in the maintenance selection process in the DSS and used in the decision nodes. Making the selection depend on 2 variables which include the RPN and availability. In this regard the normalised RPN factors down time, maintenance cost and lost utility due to failure have been accounted for; while the availability factors in component availability within operational period were equally addressed. Hence all the DGs are evaluated based on 2 main factors which are availability and system criticality based on RPN values as presented in [Table 5](https://www.sciencedirect.com/science/article/pii/S0029801823001506#tbl5).

## 4\. Case study

The [power generation system](https://www.sciencedirect.com/topics/engineering/power-generation-system "Learn more about power generation system from ScienceDirect's AI-generated Topic Pages") provides the most vital utility on board ships which suggests the level of redundancy and design resilience usually provided by ship builders. These features are common for both merchant and [naval ships](https://www.sciencedirect.com/topics/engineering/warship "Learn more about naval ships from ScienceDirect's AI-generated Topic Pages") though with significant high operational demand for the naval platforms. Failure of the [power generation](https://www.sciencedirect.com/topics/engineering/power-generation "Learn more about power generation from ScienceDirect's AI-generated Topic Pages") system for naval platforms has several implications especially considering the number of personnel onboard, and vulnerability due to loss of weapons, surveillance and habitation platforms usage. The location and type of failure are important factors to be considered in maintenance planning due to logistics and OEM related concerns. In this regard the suggested case study implements a novel methodology through the combination of reliability analysis tools to address maintenance challnegs on the power generation plant onboard and offshore patrol vessel (OPV).

Accordingly, data analysis for this research was designed to cover subjective and objective analysis. The subjective aspect of the case study provides intuitive guidance on model quality, while the objective part of the methodology provides numerical analysis using failure rates as inputs. The FMECA analysis presents experts judgement about failure and critical system component while the DFTA is a [quantitative analysis](https://www.sciencedirect.com/topics/engineering/quantitative-measurement "Learn more about quantitative analysis from ScienceDirect's AI-generated Topic Pages") on system [component reliability](https://www.sciencedirect.com/topics/earth-and-planetary-sciences/component-reliability "Learn more about component reliability from ScienceDirect's AI-generated Topic Pages"). The inputs for the BN analysis were obtained from both failure rates and MCS output of the DFT analysis, while RPN numbers from FMECA analysis were used as bases for maintenance strategy selection of individual generators. Therefore, data used for the analysis includes FMECA conducted via online survey, failure rates using [maintenance and repair](https://www.sciencedirect.com/topics/earth-and-planetary-sciences/repair-and-maintenance "Learn more about maintenance and repair from ScienceDirect's AI-generated Topic Pages") data collected from 4 marine [diesel](https://www.sciencedirect.com/topics/earth-and-planetary-sciences/diesel "Learn more about diesel from ScienceDirect's AI-generated Topic Pages") generator plants, each rated at 400 kW and can be operated parallel or individual. This was followed by discussion about operation and maintenance process onboard including wider discussion to gain expert perception on maintenance process in the fleet.

### 4.1. Subjective analysis

FMECA analysis were conducted, for the marine DGs targeted at getting expert opinion on failures mechanism and how the DGs are impact by these failures. It also provides experts judgement on how this failure affect platform availability due to issues such as, spare parts availability, technical expertise, delays due to OEM and impact of the operational environment including practices. These outcomes from the FMECA were used to generate RPN number and normalised to obtain the mission critical component, [Table 6](https://www.sciencedirect.com/science/article/pii/S0029801823001506#tbl6) shows respondents experience and assigned weights. The assign weights are product of experience in years and positions held.

Table 6. FMECA Respondents and weights.

| Positions | Respondents | Experience | Ag Weight | Applied weight (%) |
| --- | --- | --- | --- | --- |
| WKO/WKD | 2 | 0–5years | 50 + 0 | 0.5 |
| WKDWEO/MEO | 2 | 5–11 years | 60 + 0 | 0.6 |
| WEO/MEO | 4 | 11–15 years | 65 + 5 | 0.7 |
| FSWEO/FSMEO | 5 | 15–20 years | 70 + 10 | 0.8 |
| FSMO/FSG CMDR | 3 | 20–24 years | 75 + 15 | 0.9 |
| FSMO/FSG CMDR | 2 | 24–28 years | 80 + 20 | 1 |
| FSG CMDR | 2 | 28–35 years | 100 + 0 | 1 |

The FMECA survey consisted of about 20 questions on various types of faults and failure conditions covering DG system including the [alternator](https://www.sciencedirect.com/topics/engineering/alternator "Learn more about alternator from ScienceDirect's AI-generated Topic Pages"). All respondents are engineers with varying experience and specialisation. The 2 specializations are Marine and Weapon Electrical Engineers with experience levels between 5 and 35 years of service considering position occupied. The 2 variables, namely experience and specialisation were used as weights in percentages and applied to individual inputs of the respondents. Accordingly, all individual inputs were evaluated to reflect years of experience and specialisation of the respondents based on equation [(14)](https://www.sciencedirect.com/science/article/pii/S0029801823001506#fd14). Adopting the above weights, individual responses were evaluated according to experience and specialisation to obtain the population mean equation [(15)](https://www.sciencedirect.com/science/article/pii/S0029801823001506#fd15).equation 14$<math><mrow is="true"><msub is="true"><mi is="true">W</mi><mn is="true">1</mn></msub><mo linebreak="badbreak" is="true">=</mo><mrow is="true"><munderover is="true"><mo is="true">∑</mo><mrow is="true"><mi is="true">i</mi><mo linebreak="badbreak" is="true">&gt;</mo><mn is="true">0</mn></mrow><mi is="true">n</mi></munderover><mrow is="true"><msub is="true"><mi is="true">C</mi><mn is="true">1</mn></msub><mrow is="true"><mo stretchy="true" is="true">(</mo><mfrac is="true"><mrow is="true"><mi is="true">e</mi><mo linebreak="badbreak" is="true">+</mo><mi is="true">s</mi></mrow><mn is="true">100</mn></mfrac><mo stretchy="true" is="true">)</mo></mrow><mo linebreak="badbreak" is="true">+</mo></mrow></mrow><msub is="true"><mi is="true">C</mi><mn is="true">2</mn></msub><mrow is="true"><mo stretchy="true" is="true">(</mo><mfrac is="true"><mrow is="true"><mi is="true">e</mi><mo linebreak="badbreak" is="true">+</mo><mi is="true">s</mi></mrow><mn is="true">100</mn></mfrac><mo stretchy="true" is="true">)</mo></mrow><mo is="true">…</mo><mo is="true">.</mo><mo is="true">.</mo><msub is="true"><mi is="true">C</mi><mi is="true">i</mi></msub><mrow is="true"><mo stretchy="true" is="true">(</mo><mfrac is="true"><mrow is="true"><mi is="true">e</mi><mo linebreak="badbreak" is="true">+</mo><mi is="true">s</mi></mrow><mn is="true">100</mn></mfrac><mo stretchy="true" is="true">)</mo></mrow></mrow></math>$Where _W_<sub><em>1</em></sub> is the weighted component score for rank 1, n is the number of respondents in that rank, C is evaluated criterion.equation 15$<math><mrow is="true"><mi is="true">μ</mi><mo linebreak="badbreak" is="true">=</mo><mfrac is="true"><mrow is="true"><mo is="true">∑</mo><mi is="true">x</mi></mrow><mi is="true">N</mi></mfrac></mrow></math>$Where μ is the population mean, x = data values, N = number of samples.

Using the population mean for each group the weighted RPN for each subsystem and component was evaluated and normalised to ≤100 as presented in [Table 7](https://www.sciencedirect.com/science/article/pii/S0029801823001506#tbl7).

Table 7. FMECA RPN values of Most critical failures.

![](https://ars.els-cdn.com/content/image/1-s2.0-S0029801823001506-fx3.jpg)

### 4.2. Objective analysis

The objective phase of the case study provides a system reliability analysis using quantitative failure rates values of the 4 marine DGs, therefore providing a numerically objective output. The DFTA results includes component reliability, importance measures (criticality) and cut sets, which provide a significant understanding on the DGs reliability. However, it was difficult to identify specific repair, maintenance or component failure that present the highest challenge to the operators. Therefore, considering that the MCS is a combination of minimum number of events which must occur for the top event to occur (component failure); it therefor provides a good source of variables for building the BBN while taking additional inputs from the FMECA as RPN. Accordingly, this study is utilising the MCS of the DFTA to build the BN analysis by identifying top 10 most critical components as presented in [Table 7](https://www.sciencedirect.com/science/article/pii/S0029801823001506#tbl7) above.

The DFTA analysis provides both qualitative and quantitative calculations. The qualitative analysis is performed using the structure of the DFTA dependent on logic properties of the gates, while the quantitative analysis uses MRO data such as failure rate, [MTBF](https://www.sciencedirect.com/topics/earth-and-planetary-sciences/mtbf "Learn more about MTBF from ScienceDirect's AI-generated Topic Pages"), and frequency. The quantitative analysis outputs are objective results that includes system unreliability, unavailability and reliability importance measures which provide critical components failures. However, the MCS evaluation is based on the output evaluated using the logic combination of the top event occurrence usually from left to right. Therefore, to obtain the MCS the DFTA structure representing each DG was built based on the functional relationship and system boundary of the sub-systems of the respective marine DGs. MCS are product of the fault tree which forms the failure path of an evaluated fault tree, i.e the smallest set of basic events, which if they all occur will result in the top event occurring ([NASA, 2002](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib49)). However, it is important to note that a single basic event can equally form a cut set depending on the arrangement of the fault tree, [Fig. 3](https://www.sciencedirect.com/science/article/pii/S0029801823001506#fig3) provides some instance of MCS. Sub-system 1 having an AND gate fails only when all the events have occurred however the intermediate OR gate fail when any of its BEs occur, while in the case of sub-system 2, the occurrence of BE7 or BE8 is an MCS. Similarly, occurrence of any of the BEs in sub-system 3 forms a cut set for the sub-system. This highlights potential area where improvements can be achieved through alternative maintenance approach, redesign or simply altering the system to improve its reliability.

![Fig. 3](https://ars.els-cdn.com/content/image/1-s2.0-S0029801823001506-gr3.jpg)

1.  [Download : Download high-res image (164KB)](https://ars.els-cdn.com/content/image/1-s2.0-S0029801823001506-gr3_lrg.jpg "Download high-res image (164KB)")
2.  [Download : Download full-size image](https://ars.els-cdn.com/content/image/1-s2.0-S0029801823001506-gr3.jpg "Download full-size image")

Fig. 3. Instances of MCS formation.

The next step after the MCSs were obtained was to build the BBN structure using the MCS probability of occurrence as inputs to the CPT. Therefore, using a bottom-up approach, discrete chance nodes were used to model faults which are then connected to parent chance nodes representing component having probability values as inputs to the CPT, the top 10 MCS used for building the BBN are contained in [Table 8](https://www.sciencedirect.com/science/article/pii/S0029801823001506#tbl8). Overall, 8 subsystems and dependent components were modelled and analysed in the BN structure. The component chance nodes are linked to all possible faults including faults in other subsystems that could elicit multiple component failures that could result to greater maintenance or availability problems. The flexibility in BN which allows modelling CCF is very helpful in presenting complex failure interactions between components that serve many systems or subsystems.

Table 8. Top 10 cut sets for DG1-4.

| DG1 | DG 2 | DG3 | DG4 |
| --- | --- | --- | --- |
| Crankshaft journal failure | Fuel Injection pump Mechanical failure | Crankshaft journal failure | HP fuel pipe leakages |
| Fuel Filter (1&2) | FW Heat Exchanger Fouling | FW Heat Exchanger Fouling | FW Heat exchanger tube fouling |
| Sea Chest blockage | Tappet clearance (In&Ex) | RW impeller Damage | Rocker arm and Tappets clearance |
| Tappet clearance (In&Ex) | Burnt top cylinder gasket | Turbo charger lub failure | Governor drive |
| Cylinder Head sealing | Clogged Air filter | Cylinder head gasket damage | Intercooler fins fouling |
| Fuel Lift pump defects | Injector Nozzle faults | Injector nozzles faults | Turbo charger |
| Turbo Charger leakages | Clogged Air filter | blocked fuel filter | Cylinder Head gasket damage |
| Cylinder jacket cracks | Oil filter | Piston crown damage | Loose cylinder head bolts |
| Low fuel pressure | No Fuel Supply | Tappet clearance | Clogged air filter |
| RW water impeller | Defective fuel pump | Loose cylinder head bolts | Injector camshaft failure |

The process also enables more efficiently evaluation of the MCS and there impacts were more highlighted using BN analysis, hence one of the many reasons of using BN for this analysis. Moreover, the cumulative probability of the child nodes occurrence determines the operational health condition of the parent component node at the sub-system level; [Fig. 4](https://www.sciencedirect.com/science/article/pii/S0029801823001506#fig4) is a sample of the BBN structure for DG2, showing 3 out of the 8 subsystem and an abridge part of the CPT.

![Fig. 4](https://ars.els-cdn.com/content/image/1-s2.0-S0029801823001506-gr4.jpg)

1.  [Download : Download high-res image (535KB)](https://ars.els-cdn.com/content/image/1-s2.0-S0029801823001506-gr4_lrg.jpg "Download high-res image (535KB)")
2.  [Download : Download full-size image](https://ars.els-cdn.com/content/image/1-s2.0-S0029801823001506-gr4.jpg "Download full-size image")

Fig. 4. Sample BBN for DG2 showing 3 subsystems and CPT tables.

A complete model for DG 3 containing all 8 sub-systems and their nodes is shown in [Fig. 5](https://www.sciencedirect.com/science/article/pii/S0029801823001506#fig5). As can be seen in the structure the yellow nodes represent the failure cause, the light green nodes represent component failure while the sub-system are rectangle grey nodes with bar chart and the topmost blue node represents the DG. The arrows from the nodes are linked across sub-system to model common cause faults (CCF), which is typical of many [mechanical systems](https://www.sciencedirect.com/topics/engineering/mechanical-systems "Learn more about mechanical systems from ScienceDirect's AI-generated Topic Pages") especially due to temperature and load related failures. The BBN model clearly present those CCFs that can impact multiple subsystems hence provides a good reference point for the DSS.

![Fig. 5](https://ars.els-cdn.com/content/image/1-s2.0-S0029801823001506-gr5.jpg)

1.  [Download : Download high-res image (583KB)](https://ars.els-cdn.com/content/image/1-s2.0-S0029801823001506-gr5_lrg.jpg "Download high-res image (583KB)")
2.  [Download : Download full-size image](https://ars.els-cdn.com/content/image/1-s2.0-S0029801823001506-gr5.jpg "Download full-size image")

Fig. 5. BBN structure for DG 3.

The DSS was built on the existing BN structure and takes inputs from all the 8 subsystems and RPN values. Therefore, within the DSS process the non-availability of subsystem obtained in the BN is translated to reflect the ranking table in line with RPN structure as earlier presented in [Table 5](https://www.sciencedirect.com/science/article/pii/S0029801823001506#tbl5). Additional nodes namely decision and value nodes were used in conjunction with the chance nodes. The decision nodes are used to represent variables controlled by the [decision maker](https://www.sciencedirect.com/topics/engineering/decision-maker "Learn more about decision maker from ScienceDirect's AI-generated Topic Pages") while the value nodes provide a measure of the [desirability](https://www.sciencedirect.com/topics/engineering/desirability "Learn more about desirability from ScienceDirect's AI-generated Topic Pages") of the decision outcomes based on DSS process as shown in [Fig. 6](https://www.sciencedirect.com/science/article/pii/S0029801823001506#fig6).

![Fig. 6](https://ars.els-cdn.com/content/image/1-s2.0-S0029801823001506-gr6.jpg)

1.  [Download : Download high-res image (191KB)](https://ars.els-cdn.com/content/image/1-s2.0-S0029801823001506-gr6_lrg.jpg "Download high-res image (191KB)")
2.  [Download : Download full-size image](https://ars.els-cdn.com/content/image/1-s2.0-S0029801823001506-gr6.jpg "Download full-size image")

Fig. 6. DSS process schematic.

Following the above process, the DSS had 2 decision nodes with maintenance strategy and criticality level options as input. Accordingly, the value nodes had RPN values that serve as measurement of how the operators perceive the impact of failure on the DGs while ship operational availability has RPN as its inputs. In this regard the 8 chance nodes connect to value node provides the DG availability inputs in percentages while the 2 decision nodes feed in decision choices as regards the DGs availability and sub-system criticality as shown in [Fig. 7](https://www.sciencedirect.com/science/article/pii/S0029801823001506#fig7).

![Fig. 7](https://ars.els-cdn.com/content/image/1-s2.0-S0029801823001506-gr7.jpg)

1.  [Download : Download high-res image (340KB)](https://ars.els-cdn.com/content/image/1-s2.0-S0029801823001506-gr7_lrg.jpg "Download high-res image (340KB)")
2.  [Download : Download full-size image](https://ars.els-cdn.com/content/image/1-s2.0-S0029801823001506-gr7.jpg "Download full-size image")

Fig. 7. DSS structure.

## 5\. Results and discussion

This section presents the results of the analysis based on the implementation of the presented methodology and DGs in the case study. This section will start with the subjective analysis result on the FMECA output, thereafter the objective aspect will be presented to cover DFTA MCS use for BBN as well as the BN and DSS results.

### 5.1. FMECA RPN

The FMECA survey outputs provides a very important input to the overall analysis as regards what may have not been carefully accounted for in the maintenance and repair data collected from the DGs. Moreover, the MRO used was for and individual ship while the FMECA data was the response from over 20 experts with varying professional experience. Though the FMECA has in no way influenced the DFTA results it was mainly used to complement it for the second aspect of the BBN analysis which is the maintenance DSS. The fact that DFTA cannot account for issues to do with unplanned downtime, quality of replacement parts, design related unreliability and generic [human factor](https://www.sciencedirect.com/topics/engineering/ergonomics "Learn more about human factor from ScienceDirect's AI-generated Topic Pages") concern. The FMECA helps in addressing these issues as well as other environmentally induced failures which were not factored during installation but were not necessarily design related. Therefore, the FMECA survey was designed to capture some of these problems, to also highlight how the operators evaluated the most critical failures to ship availability and repairs.

It is important to understand the peculiarities and the condition of [Naval ship](https://www.sciencedirect.com/topics/engineering/warship "Learn more about Naval ship from ScienceDirect's AI-generated Topic Pages") operations, as regard [operational requirements](https://www.sciencedirect.com/topics/engineering/operational-requirement "Learn more about operational requirements from ScienceDirect's AI-generated Topic Pages") and system demand onboard. A [standard operating procedure](https://www.sciencedirect.com/topics/engineering/standard-operating-procedure "Learn more about standard operating procedure from ScienceDirect's AI-generated Topic Pages") for naval ships is parallel operation of DGs during certain exercises, navigational circumstances and load demands. In this regard, despite the redundancy in [power generation](https://www.sciencedirect.com/topics/engineering/power-generation "Learn more about power generation from ScienceDirect's AI-generated Topic Pages") system it is usual to have load shading due to high demands from other utilities. Therefore, the power generation system is one system that naval ships cannot afford to compromised.

It is therefore common to have ships undertake repairs while under way to ensure that at least 2 DGs are available, hence any repair that cannot be undertaken by ship's staff while underway is viewed as critical and can affect overall ship availability or deployments. The results of the FMECA highlights these critical failures which may not be seen as important by OEMs, but the operator's environment and operational circumstance made it so. Consequently, [Table 9](https://www.sciencedirect.com/science/article/pii/S0029801823001506#tbl9) above, is the FMECA RPN values obtained from response of operators, which details how the operators perceive the importance of DGs against the navy's operational demands, maintenance practice and capabilities including environmental conditions.

Table 9. FMECA RPN values of Most critical failures.

![](https://ars.els-cdn.com/content/image/1-s2.0-S0029801823001506-fx4.jpg)

A look at the table indicates majority of faults with low likelihood have high criticality and severity values. Therefore, taking from the definition of these 2 factors the operators are more concern with failures that affects ship availability. This is not to say safety is not of concern, in fact the threat to safety as regards loss in power generation output could be in 2 folds. First is safety and security both external and internal to the ship. The second is operational external safety to do with threats to national assets and safety of navigation which is equally a safety concern to personnel onboard. Hence, faults that can be fixed while underway or which do not expose the ship to danger i.e. loss of 2 out of the 4 DGs is within acceptable limits. Therefore, consideration for the maintenance strategy selection needs to be dynamic to reflect the prevailing operational and health condition of the DGs. Hence the need to consider the interaction between objective and subjective data sources to provide balance in the DSS analysis.

### 5.2. DFTA

MCS obtained through DFTA for individual sub systems were used as inputs to build the BN probability analysis. MCS being a combination of events or failures that leads to the system or subsystem failure can be efficiently utilised to improve system availability. Nonetheless, some failures can be triggered by a fault in another system, especially in marine diesel generators where many faults are interrelated due to system dependencies. For instance, one of the most important failures on the DGs was [crank case](https://www.sciencedirect.com/topics/engineering/crank-case "Learn more about crank case from ScienceDirect's AI-generated Topic Pages") failure but influenced by multiple factors from other subsystem such as the lubricating system and the cooling freshwater system as well as the air distribution system. This makes it difficult to isolate failure to faults, so the approach in this research is to identify the MCS, and link the components and their probability of occurrence, as shown in [Table 10](https://www.sciencedirect.com/science/article/pii/S0029801823001506#tbl10). This way the operators will be able prioritise maintenance and identify spare parts shortages as necessary.

Table 10. Top 10 cut set and probability of occurrence.

| DG1 | Probability | DG 2 | Probability | DG3 | Probability | DG4 | Probability |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Crankshaft journal failure | 0.49 | Fuel Injection pump Mechanical failure | 0.82 | Crankshaft journal failure | 0.78 | HP fuel pipe leakages | 0.85 |
| Fuel Filter (1&2) | 0.87 | FW Heat Exchanger Fouling | 0.67 | FW Heat Exchanger Fouling | 0.94 | FW Heat exchanger tube fouling | 0.7 |
| Sea Chest blockage | 0.71 | Tappet clearance (In&Ex) | 0.82 | RW impeller Damage | 0.84 | Rocker arm and Tappets clearance | 0.86 |
| Tappet clearance (In&Ex) | 0.52 | Burnt top cylinder gasket | 0.86 | Turbo charger lub failure | 0.75 | Governor drive | 0.77 |
| Cylinder Head sealing | 0.75 | Clogged Air filter | 0.75 | Cylinder head gasket damage | 0.72 | intercooler fins fouling | 0.53 |
| Fuel Lift pump defects | 0.82 | Injector Nozzle faults | 0.74 | Injector nozzles cylinder | 0.72 | Turbo charger | 0.52 |
| Turbo Charger leakages | 0.54 | Clogged Air filter | 0.76 | blacked fuel filter | 0.76 | Cylinder Head gasket damage | 0.73 |
| Cylinder jacket cracks | 0.5 | Oil filter | 0.46 | Piston crown damage | 0.87 | Loose cylinder head bolts | 0.64 |
| Low fuel pressure | 0.63 | No Fuel Supply | 0.78 | Tappet clearance | 0.8 | Clogged air filter |  |
| RW water impeller | 0.84 | Defective fuel pump | 0.82 | Loose cylinder head bolts | 0.68 | Injector camshaft failure | 0.54 |

Moreover, another important factor with MCS is that events are considered based on their contribution to failure not only occurrence. In some cases, failure occurrence may not necessarily be the reason why a component becomes critical to maintenance. In most cases factors such as [down time](https://www.sciencedirect.com/topics/engineering/downtime "Learn more about down time from ScienceDirect's AI-generated Topic Pages"), cost of repairs and repair capability could be major concerns for operators. For Instance, overheating related failures are dominated by sea water [heat exchanger](https://www.sciencedirect.com/topics/engineering/heat-exchanger "Learn more about heat exchanger from ScienceDirect's AI-generated Topic Pages") scale build-up and are mainly of concern because of the envisaged operational interruption. However, [lubrication system](https://www.sciencedirect.com/topics/engineering/lubrication-system "Learn more about lubrication system from ScienceDirect's AI-generated Topic Pages") failures or losing alternator exciter which seldom happens but their occurrence could lead to serious consequence. These types of faults are well captured by MCS formation in DFT analysis, however the DFTA structure does not support the modelling of MCS that can develop to CCF covering more than one subsystem. Hence BBN was adopted to overcome the challenge of CCF, while accommodating course effect analysis considering multiple operational factors.

### 5.3. BBN and maintenance DSS

Power generations system reliability is of great concern for all ship operators irrespective of sector, as it provides the highest utility and ensures collective safety of operators, passengers, equipment and cargo. In this regard the BBN model investigated multiple failure types and their impact on components and DG availability. Modelled components were from DFTA MCS and their failure probability from the collected MRO data. This input was used to obtain the availability for individual DGs as well as the main subsystems modelled, as shown in [Table 11](https://www.sciencedirect.com/science/article/pii/S0029801823001506#tbl11). The results shows that all 4 DGs had varying degrees of availability with DG2 being slightly more available as compared to the rest. The subsystem availability particularly that of the lubricating system of DG2 at 75% is an important pointer. Moreover, the lubricating subsystem is one of the most reliable subsystems in most DGs, this can be attributed to the centrality of its function particularly to the moving parts and heat transfer. On the other hand, a very critical situation is presented in the cooling system with availability values below 40% which is far below the expected availability of the operator. The low availability values could be linked to the sea chest blockages which can be very frequent and rapid due to scale build-up on the [cooling fins](https://www.sciencedirect.com/topics/engineering/cooling-fin "Learn more about cooling fins from ScienceDirect's AI-generated Topic Pages"). Nonetheless, the cooling system for the ships in question has at least 4 redundant sources of water supply in addition to the inline source, while this design helps reduce the risk of overheating due to delays in switching water sources. It is important that an [early warning system](https://www.sciencedirect.com/topics/engineering/early-warning-system "Learn more about early warning system from ScienceDirect's AI-generated Topic Pages") is provided to ensure that watch keepers are adequately alerted at the onset of any pressure reduction in water supply or temperature increase for at least 10 min with no corresponding increase in demands or beyond normal threshold.

Table 11. BBN DG and Component availability.

| DG | DG1 | DG2 | DG3 | DG4 | Empty Cell |
| --- | --- | --- | --- | --- | --- |
| Availability | 50% | 53% | 48% | 47% |  |
| Subsystem Availability |  |  |  |  |  |
| Cylinder Block | 47% | 43% | 44% | 44% |  |
| PTO | 60% | 56% | 50% | 60% |  |
| Cooling | 37% | 39% | 39% | 37% |  |
| Fuel System | 43% | 45% | 44% | 44% |  |
| Air Distribution | 50% | 52% | 52% | 42% |  |
| Lubrication | 62% | 75% | 56% | 55% |  |
| Inlet and Exhaust | 60% | 63% | 62% | 58% |  |
| Alternator | 59% | 52% | 59% | 57% |  |

Furthermore, the DG availability can be used independently to aid maintenance planning, in that the subsystem availability indicates where maintenance effort should be directed. However, to improve maintenance decision making additional issues that influence delivery and quality of maintenance needs to be considered. In this regard the FMECA provides a solution, and it was used for building the DSS with other inputs from the availability and maintenance strategy choices. Overall, the 4 maintenance strategy options namely Corrective Action, ConMon, PMS and Delay Action. The first 2 options are meant for high critical failures or component with severe failure consequences while the last 2 are to address failures with time dependent pattern or equipment with high redundancy and low criticality. The maintenance criticality also has 4 levels which are, Very High, High, Medium and Low and these conforms' with the maintenance strategy in order of hierarchy. The same also applies to the RPN values against the maintenance strategy options.

The approach compares the criticality in decreasing priority from very high to low based on RPN numerical values where 100 represent the highest possible outcome and 0 lowest possible outcome. The RPN values provide an iterative procedure using the linear scale ranges to place components to certain maintenance strategy group. Therefore, this helps ease some of the restriction of the component criticality Likert scale, hence providing a flexible procedure to prioritise system maintenance. Accordingly, the inputs for the overall BBN DSS comprise of the subsystem RPN, critical components and their cut set as well as the relevant CCF as shown in [Table 12](https://www.sciencedirect.com/science/article/pii/S0029801823001506#tbl12). Consequently, using these values, the DSS was built based on the structure shown in [Fig. 8](https://www.sciencedirect.com/science/article/pii/S0029801823001506#fig8) representing DG 2, showing the 3 additional nodes, 2 decision nodes on orange and 1 utility node in yellow. The decision node ‘Maintenance Decision’ is defined by independent variables of maintenance strategy choices and is a parent to Utility node which is a dependent variable and child to another decision node ‘RPN’. The decision node ‘RPN’ takes information representing the maintenance decision arrangements and matched with RPN criticality hierarchy based on RPN scale.

Table 12. Summary of BN DSS inputs.

| Sub-System | RPN | Components | MSC | CCF | Mode | Causes |
| --- | --- | --- | --- | --- | --- | --- |
| Cylinder Block | 65% | 7 | 6 | 1 | Overheating | No cooling water, lubrication oil failure, vibration, gasket damage, seizure |
| PTO | 58% | 3 | 2 | 2 | Seizure, Overheating | Missed timing, Overheating, |
| Cooling | 64% | 6 | 2 | 3 | Reduced Cooling, No cooling | Sea chest blockages, scaling, thermostat fault, Pump failure |
| Fuel System | 34% | 5 | 4 | 3 | Low Pressure, No supply, contamination | Air log, dirty tanks, filter blockage, fuel quality |
| Air Distribution | 33% | 2 | 2 | 0 | Low supply, Hot air | Air filter blockage, air cooler fouling |
| Lubrication | 3% | 3 | 2 | 0 | Low pressure, | Filter blockage, |
| No supply, | Pump failure |
| contamination | Seal failure |
| Inlet and Exhaust | 0% | 4 | 4 | 3 | Missed timing, valve clearance, poor scavenge | Valve setting/tappet clearance, weak spring, valve seat, bent valve stem |
| Alternator | 14% | 4 | 10 | 5 | Overheating, rubbing, load shedding, no output, degraded performance (low voltage/frequency) | Bearing failure. Miss alignment (lose of air gap), defective AVR, defective exciter, vibration. |

![Fig. 8](https://ars.els-cdn.com/content/image/1-s2.0-S0029801823001506-gr8.jpg)

1.  [Download : Download high-res image (330KB)](https://ars.els-cdn.com/content/image/1-s2.0-S0029801823001506-gr8_lrg.jpg "Download high-res image (330KB)")
2.  [Download : Download full-size image](https://ars.els-cdn.com/content/image/1-s2.0-S0029801823001506-gr8.jpg "Download full-size image")

Fig. 8. BBN component availability and RPN.

Following from the above, the DSS allocates percentage values between 0 and 100 to each of the 4-maintenance strategy choice for the DG based on the input data. The allocated percentage for each of the strategy determines how the maintenance action, planning and monitoring should be prioritised. This allows for flexibility regarding distribution of resources such as personnel, spare parts, logistic support and operational deployment. Furthermore, high criticality ranking for ConMon indicates the need for additional monitoring approach which can be addition of sensors, increased inspection frequency or watchkeeping attention.

The overall outcome for the maintenance strategy selection DSS of the DGs is presented in [Fig. 9](https://www.sciencedirect.com/science/article/pii/S0029801823001506#fig9). The analysis indicates how each of the DGs fit to a certain maintenance strategy regime as a reflection of the main variables i.e utility and RPN. In all, Corrective Action and ConMon appear to be the most preferred choice for all the DGs except for DG1 with relatively low figures in ConMon but high in PMS.

![Fig. 9](https://ars.els-cdn.com/content/image/1-s2.0-S0029801823001506-gr9.jpg)

1.  [Download : Download high-res image (174KB)](https://ars.els-cdn.com/content/image/1-s2.0-S0029801823001506-gr9_lrg.jpg "Download high-res image (174KB)")
2.  [Download : Download full-size image](https://ars.els-cdn.com/content/image/1-s2.0-S0029801823001506-gr9.jpg "Download full-size image")

Fig. 9. Maintenance DSS choice for all DGs.

Only DG1 and 2 seem to have some values for Delay Action and present high figures both Corrective Action and ConMon. This suggests that the 2 generators are highly maintenance intensive, moreover DG1 has about 54% to corrective action and DG2 is about 58% in ConMon. On the other hand, DG3 and DG4 fall in relatively similar level of priority levels except in PMS where DG4 numbers appear much higher than that of DG3. A likely reason for this could be that DG1 and 2 are located in the same engine room likewise DG 3 and 4. As such due to shared resources such as sea chest, ventilation, fuel line and local stress such vibration, the generators tend to present similar pattern of failure. Though some of these findings were not apparent to the operators prior to this research, however were consistent with similar research findings within the [shipping industry](https://www.sciencedirect.com/topics/engineering/shipping-industry "Learn more about shipping industry from ScienceDirect's AI-generated Topic Pages") ([Goossens and Basten, 2015](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib31); [Lazakis and Ölçer, 2015](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib41); [Lazakis et al., 2018](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib43)) and others with focus on Naval ship platforms ([Berghout et al., 2021](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib8); [Tomlinson, 2015](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bib63)).Moreover, the FMECA findings also provide additional evidence as to the [acceptability](https://www.sciencedirect.com/topics/earth-and-planetary-sciences/acceptability "Learn more about acceptability from ScienceDirect's AI-generated Topic Pages") of the research findings and relevance of the methodology.

## 6\. Conclusions

The maintenance department in large organisations help to ensure platform availability through the implementation of a maintenance strategy which fits best to organisational roles. In this regard the effort of the maintenance department is to utilise the strategy within its disposal to ensure that failures are not only minimised but are managed in an economical and timely manner. Maintenance efforts onboard also ensures that ship operators meet the IMO and ISM code provision on emission reduction and safety respectively. In this regard this research paper presented a novel methodology through the combination of reliability analysis and decision support system to help provide the most efficient maintenance strategy option for a given system and components by combining DFTA, FMECA and BBN. The methodology was implemented in the presented case study of an OPV power generation system consisting of 4 marine diesel generators. Modelled components originated from MCS obtained through DFTA and their failure probability from the collected MRO data. This input was used to obtain the availability for individual DGs as well as the main subsystems modelled. The RPN from the FMECA was generic to all generators, but the MCS from the DFTA was not, hence the DFTA cut set output was the source for the component inputs that form the child nodes in the BBN for each of the 4 DGs, while failure rates were used as inputs for the CPT. The inputs provided analytical data for ships availability analysis in BN model. The maintenance DSS was built on the existing BN with additional influence diagrams nodes taking inputs from RPN and maintenance strategy choice.

Overall, the results show that all 4 DGs had varying degrees of availability with DG2 being slightly more available as compared to the rest. The subsystem availability particularly that of the lubricating system of DG2 at 75% is an important pointer. On the other hand, a very critical situation is presented in the cooling system with availability values below 40% which is far below the expected availability of the operator pegged at 80%. The results overall indicates’ that Corrective Action and ConMon appear to be the most preferred choice for all the DGs except for the DG1 with relatively low figures in ConMon but high in PMS. Only DG1 and 2 seem to have some values for Delay Action and presented high figures both Corrective Action and ConMon. This suggests that the two generators are highly maintenance intensive, moreover DG1 has about 54% to corrective action and DG2 is about 58% in ConMon. On the other hand, DG3 and DG4 fall in relatively similar level of priority levels this except in PMS where DG4 numbers appear much higher than that of DG3. One of the major reasons for this could be that DG1 and 2 are located in the same engine room likewise DG 3 and 4, hence are affected by the same factors. Consequently, based on the outcome of the case study especially on subsystems with low availability values such as the cooling system, which can be linked to the sea chest blockages and scale built-up on the [cooling fins](https://www.sciencedirect.com/topics/engineering/cooling-fin "Learn more about cooling fins from ScienceDirect's AI-generated Topic Pages"). This is despite a relatively high redundancy in the cooling water sources, but delays in switch water sources could still lead to overheating. Therefore based on the results the following are recommended.

-   •
    
    [Early warning system](https://www.sciencedirect.com/topics/engineering/early-warning-system "Learn more about Early warning system from ScienceDirect's AI-generated Topic Pages") be provided to ensure that watch keepers are adequately alerted at the onset of any pressure reduction in water supply or temperature increase for at least 10 min with no corresponding increase in demands or beyond normal threshold.
    
-   •
    
    Provision of additional online [pressure sensors](https://www.sciencedirect.com/topics/engineering/pressure-probe "Learn more about pressure sensors from ScienceDirect's AI-generated Topic Pages") on the sea [water line](https://www.sciencedirect.com/topics/engineering/water-piping-systems "Learn more about water line from ScienceDirect's AI-generated Topic Pages").
    
-   •
    
    Consider use of cooling water additives and increased flushing frequency of the cooling water tubes.
    

Moreover, further research efforts to extend the presented work could include the use and application of [artificial neural networks](https://www.sciencedirect.com/topics/engineering/artificial-neural-network "Learn more about artificial neural networks from ScienceDirect's AI-generated Topic Pages") for fault detection and the development of a methodology for estimating the remaining useful life of ship system components, along with a spare parts estimation process. Furthermore, in light of shipping decarbonization, the application of the aforementioned methodology and tools can be extended to ship system reliability analysis on the impact of the use of biofuels and [alternative fuels](https://www.sciencedirect.com/topics/engineering/alternative-fuel "Learn more about alternative fuels from ScienceDirect's AI-generated Topic Pages") on the reliability assessment of engine system components.

## CRediT authorship contribution statement

**Abdullahi Abdulkarim Daya:** Conceptualization, Methodology, Data Collection, Software, Validation, Formal analysis, Investigation, Writing – original draft, Visualization. **Iraklis Lazakis:** Conceptualization, Validation, Resources, Writing - review, Supervision.

## Declaration of competing interest

The authors declare that they have no known competing financial interests or personal relationships that could have appeared to influence the work reported in this paper.

## Acknowledgements

The research is funded by the Federal Government of Nigeria through the Petroleum Technology Development Fund, Scholarship Number:[17PHD178](https://www.sciencedirect.com/science/article/pii/S0029801823001506#gs1)

## Data availability

Data will be made available on request.

## References

1.  [Ahn and Kurt, 2020](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bbib1)
    
    Application of a CREAM based framework to assess human reliability in emergency response to engine room fires on ships
    
2.  [Ahn et al., 2022](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bbib2)
    
    S.I. Ahn, R.E. Kurt, E. Akyuz
    
    Application of a SPAR-H based framework to assess human reliability during emergency response drill for man overboard on ships
    
3.  [A Guide to Managing Maintenance in Accordance with the Requirements of the ISM Code, 2018](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bbib70)
    
    A Guide to Managing Maintenance in Accordance with the Requirements of the ISM Code
    
    IACS (Ed.), Classisification Societies- what, why and how?, IACS (2018), pp. 1-26
    
    [Google Scholar](https://scholar.google.com/scholar?q=A%20Guide%20to%20Managing%20Maintenance%20in%20Accordance%20with%20the%20Requirements%20of%20the%20ISM%20Code%2C%20(2018).%20IACS.%20(2021).%20Class%20Monographs.%20In%20IACS%20(Ed.)%2C%20Classisification%20Societies-%20what%2C%20why%20and%20how%3F%20(pp.%201-26).)
    
4.  [Anantharaman et al., 2018](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bbib3)
    
    M. Anantharaman, F. Khan, V. Garaniya, B. Lewarn
    
    Reliability assessment of main engine subsystems considering turbocharger failure as a case study
    
    TransNav, the International Journal on Marine Navigation and Safety of Sea Transportation, 12 (2018), pp. 271-276,
    
    [10.12716/1001.12.02.06](https://doi.org/10.12716/1001.12.02.06)
    
5.  [Astrom, 2002](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bbib4)
    
    K. Astrom
    
    Simple control systems
    
    Control System Design (2002)
    
    [Google Scholar](https://scholar.google.com/scholar_lookup?title=Simple%20control%20systems&publication_year=2002&author=K.%20Astrom)
    
6.  [Bahoo et al., 2022a](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bbib5)
    
    T. Bahoo, Ahmad, M.M. Abaei, O.V. Banda, P. Kujala, F. De Carlo, R. Abbassi
    
    Prognostic Health Management of Repairable Ship Systems through Different Autonomy Degree; from Current Condition to Fully Autonomous Ship, vol. 221, Reliability Engineering & System Safety (2022),
    
    [10.1016/j.ress.2022.108355](https://doi.org/10.1016/j.ress.2022.108355)
    
7.  [Bahoo et al., 2022b](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bbib6)
    
    T. Bahoo, Ahmad, M.M. Abaei, O. Valdez Banda, J. Montewka, P. Kujala
    
    On reliability assessment of ship machinery system in different autonomy degree; A Bayesian-based approach
    
8.  [BayesFusion, 2020](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bbib7)
    
    (2020)
    
    [Google Scholar](https://scholar.google.com/scholar_lookup?title=GeNIe%20Modeler&publication_year=2020&author=BayesFusion)
    
9.  [Berghout et al., 2021](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bbib8)
    
    T. Berghout, L.-H. Mouss, T. Bentrcia, E. Elbouchikhi, M. Benbouzid
    
    A deep supervised learning approach for condition-based maintenance of naval propulsion systems
    
10.  [BS, 2010](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bbib10)
    
    EN, 13306 (2010), p. 2010
    
    [Google Scholar](https://scholar.google.com/scholar?q=BS%20EN%2013306%3A2010%2C%20(2010).)
    
11.  [Canbulat et al., 2019](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bbib11)
    
    O. Canbulat, M. Aymelek, O. Turan, E. Boulougouris
    
    An application of BBNs on the integrated energy efficiency of ship–port interface: a dry bulk shipping case
    
12.  [Ceylan et al., 2022](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bbib12)
    
    B.O. Ceylan, E. Akyuz, Y. Arslanoğlu
    
    Modified quantitative systems theoretic accident model and processes (STAMP) analysis: a catastrophic ship engine failure case
    
13.  [Cheliotis et al., 2020](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bbib13)
    
    M. Cheliotis, I. Lazakis, G. Theotokatos
    
    Machine learning and data-driven fault detection for ship systems operations
    
14.  [Chen et al., 2021](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bbib14)
    
    L. Chen, Y. Gao, H. Dui, L. Xing
    
    Importance Measure-Based Maintenance Optimization Strategy for Pod Slewing System, vol. 216, Reliability Engineering & System Safety (2021),
    
    [10.1016/j.ress.2021.108001](https://doi.org/10.1016/j.ress.2021.108001)
    
15.  [Chiacchio et al., 2016](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bbib15)
    
    F. Chiacchio, D. D'Urso, L. Compagno, M. Pennisi, F. Pappalardo, G. Manno
    
    SHyFTA, a Stochastic Hybrid Fault Tree Automaton for the modelling and simulation of dynamic reliability problems
    
16.  [Cicek and Celik, 2013](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bbib16)
    
    K. Cicek, M. Celik
    
    Application of failure modes and effects analysis to main engine crankcase explosion failure on-board ship
    
17.  [Cipollini et al., 2018](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bbib17)
    
    F. Cipollini, L. Oneto, A. Coraddu, A.J. Murphy, D. Anguita
    
    Condition-based maintenance of naval propulsion systems with supervised data analysis
    
18.  [Codetta-Raiteri and Portinale, 2017](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bbib19)
    
    D. Codetta-Raiteri, L. Portinale
    
    Generalized Continuous Time Bayesian Networks as a modelling and analysis formalism for dependable systems
    
19.  [Daya, 2021](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bbib20)
    
    A.A. Daya, I. L.
    
    _Application Of Artifical Neural Network and Dynamic Fault Tree Analysis to Enhance Reliability in Predictive Ship Machinery Health Condition Monitoring_ GMO-SHIPMAR
    
    (2021)
    
    [Google Scholar](https://scholar.google.com/scholar_lookup?title=Application%20Of%20Artifical%20Neural%20Network%20and%20Dynamic%20Fault%20Tree%20Analysis%20to%20Enhance%20Reliability%20in%20Predictive%20Ship%20Machinery%20Health%20Condition%20Monitoring%20GMO-SHIPMAR&publication_year=2021&author=A.A.%20Daya&author=I.%20L.)
    
20.  [Daya and Lazakis, 2022](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bbib21)
    
    A.A. Daya, I. Lazakis
    
    Investigating ship system performance degradation and failure criticality using FMECA and Artificial Neural Networks
    
21.  [DoD, 1980](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bbib22)
    
    (1980)
    
    [Google Scholar](https://scholar.google.com/scholar_lookup?title=MIL%20STD%201629A&publication_year=1980&author=U.S.%20DoD)
    
22.  [DoD, 1989](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bbib23)
    
    Maintainability Program for Systems and Equipment (1989)
    
    [Google Scholar](https://scholar.google.com/scholar_lookup?title=MIL-STD-470B&publication_year=1989&author=DoD)
    
23.  [DoD, 2005](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bbib24)
    
    DoD
    
    Reliability Availability and Maintainability (RAM) Guide
    
    (2005)
    
    [Google Scholar](https://scholar.google.com/scholar_lookup?title=Reliability%20Availability%20and%20Maintainability%20%20Guide&publication_year=2005&author=DoD)
    
24.  [Duan and Zhou, 2012](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bbib25)
    
    R.-x. Duan, H.-l. Zhou
    
    A new fault diagnosis method based on Fault Tree and bayesian networks
    
25.  [Eriksen et al., 2021](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bbib27)
    
    S. Eriksen, I.B. Utne, M. Lützen
    
    An RCM Approach for Assessing Reliability Challenges and Maintenance Needs of Unmanned Cargo Ships, vol. 210, Reliability Engineering & System Safety (2021),
    
    [10.1016/j.ress.2021.107550](https://doi.org/10.1016/j.ress.2021.107550)
    
26.  [Fu et al., 2022](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bbib28)
    
    S. Fu, Y. Yu, J. Chen, B. Han, Z. Wu
    
    Towards a probabilistic approach for risk analysis of nuclear-powered icebreakers using FMEA and FRAM
    
27.  [F.V et al., 2007](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bbib29)
    
    Jensen F.V, T. D. N
    
    Bayesian Networks and Decision Graphs
    
    (2 ed.), Springer (2007)
    
    [Google Scholar](https://scholar.google.com/scholar_lookup?title=Bayesian%20Networks%20and%20Decision%20Graphs&publication_year=2007&author=Jensen%20F.V&author=T.%20D.%20N)
    
28.  [Galagedarage Don and Khan, 2019](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bbib30)
    
    M. Galagedarage Don, F. Khan
    
    Dynamic process fault detection and diagnosis based on a combined approach of hidden Markov and Bayesian network model
    
29.  [Goossens and Basten, 2015](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bbib31)
    
    A.J.M. Goossens, R.J.I. Basten
    
    Exploring maintenance policy selection using the Analytic Hierarchy Process; an application for naval ships
    
30.  [Jakkula et al., 2020](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bbib32)
    
    B. Jakkula, G.R. Mandela, M. Ch S N
    
    Reliability block diagram (RBD) and fault tree analysis (FTA) approaches for estimation of system reliability and availability – a case study
    
31.  [Jeong et al., 2018](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bbib33)
    
    B. Jeong, B.S. Lee, P. Zhou, S.-m. Ha
    
    Quantitative risk assessment of medium-sized floating regasification units using system hierarchical modelling
    
32.  [Jun and Kim, 2017](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bbib34)
    
    H.-B. Jun, D. Kim
    
    A Bayesian network-based approach for fault analysis
    
33.  [Kabir, 2017](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bbib35)
    
    S. Kabir
    
    An overview of fault tree analysis and its application in model based dependability analysis
    
34.  [Kabir and Papadopoulos, 2019](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bbib36)
    
    S. Kabir, Y. Papadopoulos
    
    Applications of Bayesian networks and Petri nets in safety, reliability, and risk assessments: a review
    
35.  [Kampitsis and Panagiotidou, 2022](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bbib37)
    
    D. Kampitsis, S. Panagiotidou
    
    A Bayesian Condition-Based Maintenance and Monitoring Policy with Variable Sampling Intervals, vol. 218, Reliability Engineering & System Safety (2022),
    
    [10.1016/j.ress.2021.108159](https://doi.org/10.1016/j.ress.2021.108159)
    
36.  [Karatuğ and Arslanoğlu, 2022](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bbib38)
    
    Ç. Karatuğ, Y. Arslanoğlu
    
    Development of condition-based maintenance strategy for fault diagnosis for ship engine systems
    
37.  [Khakzad et al., 2011](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bbib39)
    
    N. Khakzad, F. Khan, P. Amyotte
    
    Safety analysis in process facilities: comparison of fault tree and Bayesian network approaches
    
38.  [Konstantinos Dikis et al., 2010](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bbib40)
    
    I.L. Konstantinos Dikis, Atabak Taheri, Gerasimos Theotokatos
    
    Risk and Reliability Analysis Tool Development for Ship Machinery
    
    (2010)
    
    [Google Scholar](https://scholar.google.com/scholar_lookup?title=Risk%20and%20Reliability%20Analysis%20Tool%20Development%20for%20Ship%20Machinery&publication_year=2010&author=I.L.%20Konstantinos%20Dikis&author=Atabak%20Taheri&author=Gerasimos%20Theotokatos)
    
39.  [Kuzu and Senol, 2021](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bbib73)
    
    A.C. Kuzu, Y.E. Senol, Maintenace Management in Surface Ships, (2007)
    
    Fault tree analysis of cargo leakage from manifold connection in fuzzy environment: A novel case of anhydrous ammonia
    
    Ocean Eng. (2021), p. 238
    
    [https://doi.org/10.1016/j.oceaneng.2021.109720BR1313-](https://doi.org/10.1016/j.oceaneng.2021.109720BR1313-)
    
    [Google Scholar](https://scholar.google.com/scholar_lookup?title=Fault%20tree%20analysis%20of%20cargo%20leakage%20from%20manifold%20connection%20in%20fuzzy%20environment%3A%20A%20novel%20case%20of%20anhydrous%20ammonia&publication_year=2021&author=A.C.%20Kuzu&author=Y.E.%20Senol&author=Maintenace%20Management%20in%20Surface%20Ships%2C%20(2007))
    
40.  [Lazakis and Ölçer, 2015](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bbib41)
    
    I. Lazakis, A. Ölçer
    
    Selection of the best maintenance approach in the maritime industry under fuzzy multiple attributive group decision-making environment
    
41.  [Lazakis et al., 2010](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bbib42)
    
    I. Lazakis, O. Turan, S. Aksu
    
    Increasing ship operational reliability through the implementation of a holistic maintenance management strategy
    
42.  [Lazakis et al., 2018](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bbib43)
    
    I. Lazakis, Y. Raptodimos, T. Varelas
    
    Predicting ship machinery system condition through analytical reliability tools and artificial neural networks
    
43.  [Leimeister and Kolios, 2018](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bbib44)
    
    M. Leimeister, A. Kolios
    
    A review of reliability-based methods for risk analysis and their application in the offshore wind industry
    
44.  [Li et al., 2020](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bbib45)
    
    H. Li, C. Guedes Soares, H.-Z. Huang
    
    Reliability analysis of a floating offshore wind turbine using Bayesian Networks
    
45.  [Marko Cerpin, 2002](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bbib46)
    
    B.M. Marko Cerpin
    
    A Dynamic Fault Tree
    
    Reliability Engineering & System Safety (2002)
    
    [Google Scholar](https://scholar.google.com/scholar_lookup?title=A%20Dynamic%20Fault%20Tree&publication_year=2002&author=B.M.%20Marko%20Cerpin)
    
46.  [Marving Rausand and Arnljot, 2021](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bbib47)
    
    A.B. Marving Rausand, Hoyland Arnljot
    
    System Reliability Theory Models,Statistical Methods and Application
    
    Wiley (2021)
    
    [Google Scholar](https://scholar.google.com/scholar_lookup?title=System%20Reliability%20Theory%20Models%2CStatistical%20Methods%20and%20Application&publication_year=2021&author=A.B.%20Marving%20Rausand&author=Hoyland%20Arnljot)
    
47.  [Melani et al., 2018](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bbib48)
    
    A.H.A. Melani, C.A. Murad, A. Caminada Netto, G.F. M.d. Souza, S.I. Nabeta
    
    Criticality-based maintenance of a coal-fired power plant
    
48.  [NASA, 2002](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bbib49)
    
    NASA
    
    Fault Tree Handbook with Aerospace Applications
    
    (2002)
    
    [Google Scholar](https://scholar.google.com/scholar_lookup?title=Fault%20Tree%20Handbook%20with%20Aerospace%20Applications&publication_year=2002&author=NASA)
    
49.  [NASA, 2008](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bbib50)
    
    (2008)
    
    [Google Scholar](https://scholar.google.com/scholar_lookup?title=RCM%20Guide&publication_year=2008&author=NASA)
    
50.  [NAVSEA, 2007](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bbib51)
    
    (2007)
    
    [Google Scholar](https://scholar.google.com/scholar_lookup?title=RCM%20Handbook&publication_year=2007&author=NAVSEA)
    
51.  [Niculita et al., 2017](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bbib52)
    
    O. Niculita, O. Nwora, Z. Skaf
    
    Towards design of prognostics and health management solutions for maritime assets
    
52.  [NSWC, 2011](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bbib53)
    
    NSWC
    
    NSWC-11 RELIABILITY HDBK
    
    (2011)
    
    [Google Scholar](https://scholar.google.com/scholar_lookup?title=NSWC-11%20RELIABILITY%20HDBK&publication_year=2011&author=NSWC)
    
53.  [Piadeh et al., 2018](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bbib54)
    
    F. Piadeh, M. Ahmadi, K. Behzadian
    
    Reliability assessment for hybrid systems of advanced treatment units of industrial wastewater reuse using combined event tree and fuzzy fault tree analyses
    
54.  [Relex et al., 2003](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bbib55)
    
    B. Bangalore, D. Paul, P. Geoff, S. Ron, W. Robert (Eds.), Reliability: Practitioner's Guide, Relex Software Corpoaration (2003)
    
    [Google Scholar](https://scholar.google.com/scholar_lookup?title=Reliability%3A%20Practitioners%20Guide&publication_year=2003&author=Relax)
    
55.  [Ruijters and Stoelinga, 2015](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bbib56)
    
    E. Ruijters, M. Stoelinga
    
    Fault tree analysis: a survey of the state-of-the-art in modeling, analysis and tools
    
56.  [Saaty, 2016](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bbib57)
    
    T.L. Saaty
    
    The analytic hierarchy and analytic network processes for the measurement of intangible criteria and for decision-making
    
57.  [Sakar et al., 2021](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bbib58)
    
    C. Sakar, A.C. Toz, M. Buber, B. Koseoglu
    
    Risk analysis of grounding accidents by mapping a Fault Tree into a bayesian network
    
58.  [Scheu et al., 2017](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bbib59)
    
    M.N. Scheu, A. Kolios, T. Fischer, F. Brennan
    
    Influence of statistical uncertainty of component reliability estimations on offshore wind farm availability
    
59.  [Shafiee et al., 2016](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bbib60)
    
    M. Shafiee, I. Animah, N. Simms
    
    Development of a techno-economic framework for life extension decision making of safety critical installations
    
60.  [Soliman, 2020](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bbib61)
    
    M.H. Soliman
    
    _Machine Reliability and Condition Monitoring A Comprehensive Guide to Predictive Maintenance Planing_ \[Literature
    
61.  [Tan et al., 2011](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bbib62)
    
    Z. Tan, J. Li, Z. Wu, J. Zheng, W. He
    
    An evaluation of maintenance strategy using risk based inspection
    
62.  [Tomlinson, 2015](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bbib63)
    
    N. Tomlinson
    
    Whats Is the Ideal Maintenance Strategy- A Look at Both MoD and Commercial Shipping Best Practice
    
    (2015)
    
    [Google Scholar](https://scholar.google.com/scholar_lookup?title=Whats%20Is%20the%20Ideal%20Maintenance%20Strategy-%20A%20Look%20at%20Both%20MoD%20and%20Commercial%20Shipping%20Best%20Practice&publication_year=2015&author=N.%20Tomlinson)
    
63.  [Turan et al., 2012](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bbib64)
    
    O. Turan, I. Lazakis, S. Judah
    
    _Establishing the Optimum Vessel Maintenance Approach Based On System Reliability and Criticality Analysis_ Managing Reliability & Maintainability in the
    
    Maritime Industry (2012)
    
    [Google Scholar](https://scholar.google.com/scholar_lookup?title=Establishing%20the%20Optimum%20Vessel%20Maintenance%20Approach%20Based%20On%20System%20Reliability%20and%20Criticality%20Analysis%20Managing%20Reliability%20%20Maintainability%20in%20the&publication_year=2012&author=O.%20Turan&author=I.%20Lazakis&author=S.%20Judah)
    
64.  [Velasco-Gallego and Lazakis, 2022a](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bbib65)
    
    C. Velasco-Gallego, I. Lazakis
    
    RADIS: a real-time anomaly detection intelligent system for fault diagnosis of marine machinery
    
65.  [Velasco-Gallego and Lazakis, 2022b](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bbib66)
    
    C. Velasco-Gallego, I. Lazakis
    
    A real-time data-driven framework for the identification of steady states of marine machinery
    
66.  [Weber et al., 2012](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bbib67)
    
    P. Weber, G. M.-O, C. Simon, B. Lung
    
    Overview on Bayesian Network Applications for Dependability , Risk Analysis and Maintenance Areas. _Engineering Applications Of Artificial Intelligence_
    
    (2012)
    
    [Google Scholar](https://scholar.google.com/scholar_lookup?title=Overview%20on%20Bayesian%20Network%20Applications%20for%20Dependability%20%2C%20Risk%20Analysis%20and%20Maintenance%20Areas.%20Engineering%20Applications%20Of%20Artificial%20Intelligence&publication_year=2012&author=P.%20Weber&author=G.%20M.-O&author=C.%20Simon&author=B.%20Lung)
    
67.  [Zhou et al., 2022](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bbib68)
    
    S. Zhou, L. Ye, S. Xiong, J. Xiang
    
    Reliability Analysis of Dynamic Fault Trees with Priority-AND Gates Based on Irrelevance Coverage Model, vol. 224, Reliability Engineering & System Safety (2022),
    
    [10.1016/j.ress.2022.108553](https://doi.org/10.1016/j.ress.2022.108553)
    

## Further readings

1.  [Class Monographs., 2018](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bfur1)
    
    Class Monographs., IACS (Ed.), 2018. 1-26).Prohibition on the Carriege of non-compliant oil for combustion onboard a ship.
    
    [Google Scholar](https://scholar.google.com/scholar?q=Class%20Monographs.%2C%20IACS%20(Ed.)%2C%202018.%201-26).Prohibition%20on%20the%20Carriege%20of%20non-compliant%20oil%20for%20combustion%20onboard%20a%20ship.)
    

1.  [Equipment Condition Monitoring, 2016](https://www.sciencedirect.com/science/article/pii/S0029801823001506#bfur2)
    
    Equipment Condition Monitoring, (2016). Bahoo, T., Ahmad, Abaei, M.M., Valdez Banda, O., Montewka, J., & Kujala, P. (2022). On reliability assessment of ship machinery system in different autonomy degree; A Bayesian-based approach. Ocean Eng., 254. [https://doi.org/10.1016/j.oceaneng.2022.111252IACS](https://doi.org/10.1016/j.oceaneng.2022.111252IACS). (2021).
    
    [Google Scholar](https://scholar.google.com/scholar?q=Equipment%20Condition%20Monitoring%2C%20(2016).%20Bahoo%2C%20T.%2C%20Ahmad%2C%20Abaei%2C%20M.M.%2C%20Valdez%20Banda%2C%20O.%2C%20Montewka%2C%20J.%2C%20%26%20Kujala%2C%20P.%20(2022).%20On%20reliability%20assessment%20of%20ship%20machinery%20system%20in%20different%20autonomy%20degree%3B%20A%20Bayesian-based%20approach.%20Ocean%20Eng.%2C%20254.%20https%3A%2F%2Fdoi.org%2F10.1016%2Fj.oceaneng.2022.111252IACS.%20(2021).)
    

## Cited by (0)

© 2023 The Authors. Published by Elsevier Ltd.
