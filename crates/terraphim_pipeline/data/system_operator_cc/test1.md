---
created: 2023-09-21T15:56:43 (UTC +01:00)
tags: []
source: https://www.sciencedirect.com/science/article/pii/S0736584522001673
author: M.A. Jacobs, M. Swink
---

# A personalised operation and maintenance approach for complex products based on equipment portrait of product-service system - ScienceDirect

> ## Excerpt
> Based on the holistic data of product-service system (PSS) delivery processes, equipment portrait can be used to describe personalised user requiremen…

---
[![Elsevier](https://sdfestaticassets-eu-west-1.sciencedirectassets.com/prod/c5ec5024630bc984ae859b0b2315edad4a342b5a/image/elsevier-non-solus.png)](https://www.sciencedirect.com/journal/robotics-and-computer-integrated-manufacturing "Go to Robotics and Computer-Integrated Manufacturing on ScienceDirect")

[![Robotics and Computer-Integrated Manufacturing](https://ars.els-cdn.com/content/image/1-s2.0-S0736584522X00057-cov150h.gif)](https://www.sciencedirect.com/journal/robotics-and-computer-integrated-manufacturing/vol/80/suppl/C)

[https://doi.org/10.1016/j.rcim.2022.102485](https://doi.org/10.1016/j.rcim.2022.102485 "Persistent link using digital object identifier")[Get rights and content](https://s100.copyright.com/AppDispatchServlet?publisherName=ELS&contentID=S0736584522001673&orderBeanReset=true)

## Highlights

-   •
    
    A personalised [OM](https://www.sciencedirect.com/topics/engineering/operation-and-maintenance "Learn more about OM from ScienceDirect's AI-generated Topic Pages") approach for complex products is proposed.
    
-   •
    
    A framework of POMA-CP based on equipment portrait within the PSS is developed.
    
-   •
    
    A multi-level case library and dynamic equipment portrait model are established.
    
-   •
    
    The feasibility of the proposed method is verified through an application scenario.
    

## Abstract

Based on the holistic data of product-service system (PSS) delivery processes, equipment portrait can be used to describe personalised user requirements and conduct targeted analysis on the performance of complex products. Therefore, a promising application combining PSS and equipment portrait is to establish a more refined portrait model to improve the accuracy and applicability of operation and maintenance (OM) schemes for industrial products. However, studies in the above field are facing many challenges. For example, the research on equipment portrait in the industrial field is still in its infancy. PSS and equipment portrait are studied separately, and the overall solution that integrates PSS and equipment portrait for complex products OM service is almost vacant. A personalised OM approach for complex products (POMA-CP) is proposed to address these challenges. First, a framework of POMA-CP is developed to show how the processes of refined OM can be implemented. Then, a solution of POMA-CP based on the framework is designed. A multi-level case library, dynamic equipment portrait model, and case-pushing mechanism are established and developed. Active pushing of the best similar cases and automatic generation of service schemes are realised. Finally, an application scenario for a high-speed electric multiple units (EMU) bogie is presented to illustrate the feasibility and effectiveness of the proposed approach. Higher accuracy and applicability for service schemes are achieved, resulting in the efficient reusing of OM knowledge, proactive implementation of refined maintenance, and reducing maintenance cost and resource consumption.

-   [Previous](https://www.sciencedirect.com/science/article/pii/S073658452200165X)
-   [Next](https://www.sciencedirect.com/science/article/pii/S0736584522001715)

## Keywords

Equipment portrait

Product-service system

Operation and maintenance

Personalised service

## Nomenclature

_attr<sub>j</sub>_

The _j_\-th fault symptom label

_attr<sub>r</sub>_

The _r_\-th fault symptom keyword

_BT<sub>j</sub>_

The time when _attr<sub>j</sub>_ is added to _F<sub>i</sub>_

_CASE_

OM service case

_CCS_

Critical component systems

_E<sub>i</sub>_

The attribute and behaviour feature model

_FC_

Fault category of _[CCS](https://www.sciencedirect.com/topics/engineering/carbon-capture-and-storage "Learn more about CCS from ScienceDirect's AI-generated Topic Pages")_

_F<sub>i</sub>_

The fault symptom feature model

_FSK_

Fault symptom keywords

_GS_()

Global similarity of _E<sub>i</sub>_ and

_GS_()

Global similarity of _F<sub>i</sub>_ and

_H_(_FC_ | _arrr<sub>j</sub>_)

[Conditional entropy](https://www.sciencedirect.com/topics/engineering/conditional-entropy "Learn more about Conditional entropy from ScienceDirect's AI-generated Topic Pages") of _attr<sub>j</sub>_ for _FC_

_H<sub>j</sub>_

The lower bounds of the interval number

_IDF_ (_arrr<sub>r</sub>_)

[Inverse document frequency](https://www.sciencedirect.com/topics/computer-science/inverse-document-frequency "Learn more about Inverse document frequency from ScienceDirect's AI-generated Topic Pages") of _attr<sub>r</sub>_

_JS_()

Structural similarity of _E<sub>i</sub>_ and

_JS_()

Structural similarity of _F<sub>i</sub>_ and

_L<sub>j</sub>_

The upper bounds of the interval number

_LS_()

Local similarity between _E<sub>i</sub>_ and

_LS_()

Local similarity between _F<sub>i</sub>_ and

_LT<sub>j</sub>_

The time when _attr<sub>j</sub>_ is last updated in _F<sub>i</sub>_

_MS_

Maintenance scheme

_M<sub>u</sub>_

Equipment portrait model

_N_ (_attr<sub>j</sub>, FC<sub>i</sub>_)

The occurred times of _attr<sub>j</sub>_ in _FC<sub>i</sub>_

The number of attributes for the [intersection set](https://www.sciencedirect.com/topics/computer-science/intersection-set "Learn more about intersection set from ScienceDirect's AI-generated Topic Pages") of _E<sub>i</sub>_ and

The number of attributes for the union set of _E<sub>i</sub>_ and

The number of attributes for the intersection set of _F<sub>i</sub>_ and

The number of attributes for the union set of _F<sub>i</sub>_ and

_num<sub>E</sub>_

The number of common attribute and behaviour labels

_num<sub>F</sub>_

The number of common fault symptom labels

_P_(_FC<sub>i</sub>_ | _arrr<sub>j</sub>_)

The probability of _FC<sub>i</sub>_ when the _attr<sub>j</sub>_ appears

_P_(_FC<sub>z</sub>_<sub>,</sub>_<sub>i</sub>_)

The probability of a case in the _CASE_ library belongs to _FC<sub>z,i</sub>_

_p<sub>i</sub>_

The total number of all _CASE_ in _FC<sub>z,i</sub>_

_p<sub>i</sub>_<sub>,</sub>_<sub>r</sub>_

The number of _CASE_ contain _attr<sub>r</sub>_ in the _FC<sub>z,i</sub>_

_P<sub>ω</sub>_(_arrr<sub>j</sub>, FC<sub>i</sub>_)

The local weight of _attr<sub>j</sub>_ for _FC<sub>i</sub>_

_S_

Fault symptom feature vector of _CASE_

_SCL_

Sub-case library of _CCS_

_T_

Fault feature word

_TF_(_arrr<sub>r</sub>_)

Term frequency of _attr<sub>r</sub>_

_T<sub>ω</sub>_(_arrr<sub>j</sub>_)

The global weight of _attr<sub>j</sub>_ for a _FC_ set

The values of _E<sub>i</sub>_

The values of

_W<sub>h</sub>_

The content of attribute or behaviour label

_Z_

The number of _CCS_

_η<sub>j</sub>_

The weight of _j_\-th fault symptom label

_η<sub>r</sub>_

The weight of _r_\-th fault symptom keywords

_η<sub>threshold</sub>_

The feature selection weight of _attr<sub>r</sub>_

_ρ<sub>E</sub>_

The weight of _E<sub>i</sub>_

_ρ<sub>F</sub>_

The weight of _F<sub>i</sub>_

_ρ<sub>j</sub>_

The weight of common label

_χ_(_T_)<sup>2</sup>

The chi-square value of _T_ relative to the whole _CASE_ library of the _CCS<sub>z</sub>_

_χ_(_T, FC<sub>z</sub>_<sub>,</sub>_<sub>i</sub>_)<sup>2</sup>

The chi-square value between _T_ and _FC<sub>z,i</sub>_

_ω<sub>i</sub>_

The weight of fault category feature vector

### Abbreviation

B2B

Business to Business

BOM

Bill of material

CE

Circular economy

CP

Cleaner production

EMU

Electric multiple units

FMEA

Failure mode & effect analysis

FRACAS

Failure reporting analysis and corrective action system

IoT

Internet of Things

OEMs

Original equipment manufacturers

OM

Operation and maintenance

POMA-CP

Personalised OM approach for complex products

PSS

Product-service system

R&D

Research and development

RFID

[Radio frequency identification](https://www.sciencedirect.com/topics/engineering/radio-frequency-identification "Learn more about Radio frequency identification from ScienceDirect's AI-generated Topic Pages")

RQ

Research question

TF-IDF

Term frequency-inverse document frequency

## 1\. Introduction

Product complexity has been described in the literature as having many dimensions (including the number of components, the number of modules, the diversity of relations between components, and the commonality of products in an assortment) [\[1\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0001), [\[2\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0002), [\[3\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0003), while complex products are usually defined as a durable product with a lifespan of over 10–30 years [\[4\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0004). In addition, complex products also have the characteristics of complex structure, long research and development (R&D) cycle, strict quality control and high maintenance service requirements \[[5](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0005),[6](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0006)\]. Therefore, complex products’ [operation and maintenance](https://www.sciencedirect.com/topics/engineering/operation-and-maintenance "Learn more about operation and maintenance from ScienceDirect's AI-generated Topic Pages") (OM) service can provide huge potential for establishing a balance amongst users, [original equipment manufacturers](https://www.sciencedirect.com/topics/engineering/original-equipment-manufacturer "Learn more about original equipment manufacturers from ScienceDirect's AI-generated Topic Pages") (OEMs), and natural environment \[[7](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0007),[8](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0008)\]. Complex products’ personalised OM service demands are emerging with the ever-increasing usage of customised products. As a result, many OEMs are making efforts to employ the product-service system (PSS) to provide high value-added service for their users to improve customer satisfaction and promote the implementation of cleaner production (CP) and circular economy (CE) strategies \[[\[9\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0009), [\[10\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0010), [\[11\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib69)\].

With the advancement of smart technologies in the [Industry 4.0](https://www.sciencedirect.com/topics/computer-science/industry-4-0 "Learn more about Industry 4.0 from ScienceDirect's AI-generated Topic Pages") paradigm, the research on integrating PSS with OM services has attracted many scholars’ attention. As pointed out by Zheng et al. \[[12](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0011),[13](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0012)\], the combination of PSS and smart technologies can enable innovative applications of OM service for complex products. Exner et al. \[[14](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0013)\] analysed how OM services can be connected to the PSS to enhance machine availability while [reducing energy consumption](https://www.sciencedirect.com/topics/engineering/reducing-energy-consumption "Learn more about reducing energy consumption from ScienceDirect's AI-generated Topic Pages") and the cost of maintenance procedures. D. Mourtzis et al. \[[15](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0014)\] studied an unscheduled OM service of manufacturing equipment following the PSS. A maintenance planning system was proposed to facilitate [knowledge sharing](https://www.sciencedirect.com/topics/computer-science/knowledge-sharing "Learn more about knowledge sharing from ScienceDirect's AI-generated Topic Pages") amongst OEMs, users and service providers within the PSS \[[16](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0015)\]. Recent research achievements, such as PSS-based maintenance grouping strategy \[[17](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0016)\], process-based OM service [knowledge management](https://www.sciencedirect.com/topics/computer-science/knowledge-management "Learn more about knowledge management from ScienceDirect's AI-generated Topic Pages") mechanism \[[18](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0017)\], sustainable service-orientated equipment maintenance \[[19](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0018)\], active [preventive maintenance](https://www.sciencedirect.com/topics/engineering/preventive-maintenance "Learn more about preventive maintenance from ScienceDirect's AI-generated Topic Pages") with lifecycle big data \[[6](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0006),[8](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0008),[20](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0019)\], and lease-orientated opportunistic maintenance \[[21](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0020)\], etc., have provided significant references for industrial practitioners to develop and apply the PSS-based product OM pattern.

However, the PSS-based OM services face new issues while the increase in product complexity and industrial automation. For instance, how to effectively integrate the scattered and multi-stage stakeholders and [lifecycle data](https://www.sciencedirect.com/topics/engineering/lifecycle-data "Learn more about lifecycle data from ScienceDirect's AI-generated Topic Pages"), rationally apply the historical OM service knowledge, and facilitate the OEMs to carry out personalised and refined OM services. Fortunately, equipment portrait, as an emerging technology derived from user portrait, can accurately describe personalised user needs \[[22](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0021)\] and construct fine-grained portrait models to analyse complex products’ operational status and fault symptoms \[[23](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0022)\]. Besides, equipment portrait has achieved initial success in software product development and maintenance \[[24](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0023)\], machine tools intelligent maintenance \[[25](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0024)\], and water supply infrastructures OM service \[[26](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0025)\]. Therefore, the equipment portrait shows great potential for addressing the above issues and realising personalised and refined product OM services.

Despite the progress achieved by the researchers in the abovementioned work, new questions are arising from applying equipment portrait to personalised OM service of complex products in the [big data environment](https://www.sciencedirect.com/topics/computer-science/big-data-environment "Learn more about big data environment from ScienceDirect's AI-generated Topic Pages") and Industry 4.0 paradigm:

First, the core of equipment portrait is extracting the set of feature labels. However, the lifecycle data of complex products is usually characterized by high volume, high velocity and high variety \[[27](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0026)\]. As a result, it is difficult for industrial practitioners to effectively define different feature labels of products according to specific application scenarios when they are not clear on how to use the lifecycle data best \[[28](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0027)\]. That is, the prescriptive and structured processes for the establishment of equipment portrait models for complex products to make OM service decisions were seldom investigated. As pointed out by Liu et al. \[[29](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0028)\], a promising application of equipment portrait in the industrial field is the use of its model to conduct a targeted analysis of the performance of products and systems. Therefore, the authors highlight an important research question (RQ1): how should an equipment portrait-based framework for OM service delivery and execution process of industrial complex products be structured to exploit the values of portrait technology in product OM service activities?

Second, current research on product OM mostly combines either equipment portrait and maintenance service \[[30](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0029)\] or aligns PSS mode with maintenance policy planning \[[17](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0016),[31](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0030)\]. As pointed out by Grover et al. \[[32](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0031)\], the interaction of accessible product data and efficient [data analytics](https://www.sciencedirect.com/topics/computer-science/data-analytics "Learn more about data analytics from ScienceDirect's AI-generated Topic Pages") and appropriate business model can obtain more insights to improve maintenance services. However, virtually no literature provides knowledge of exploiting equipment portrait and PSS to bring out value from lifecycle data, and to carry out more precisely and prescriptive maintenance measures \[[33](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0032),[34](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0033)\]. This resulted in the industrial practitioners being unable to comprehensively and accurately assess the operational conditions of complex products, which makes it difficult to fulfil the current requirements of personalised and refined product OM services. This limitation stimulated another research question (RQ2): what methods can be applied to effectively integrate portrait technologies and PSS mode to extract knowledge from PSS delivery process data and to define feature labels, so as to support an equipment portrait-based decision for the OM service delivery and execution process?

This paper proposed a personalised OM approach for complex products (POMA-CP) based on equipment portrait within the PSS mode to answer both of the above questions. The proposed method can contribute to constructing a more accurate equipment portrait model based on the comprehensive, meticulous and personalised requirement information of complex products OM services. It is important to emphasise that, with the wide application of smart technologies in Industry 4.0, the large amount of data related to the product's operational status and the whole lifecycle captured in the PSS delivery processes are valuable assets for extracting the above information. Therefore, the combined PSS and equipment portrait application is not simply a study of innovative product management but provides a new paradigm for realising the preventive and refined as well as personalised OM service for complex products. The proposed approach has strong flexibility and adaptability toward complex products’ OM services in big data and Industry 4.0 environments. As a result, two such technologies incorporate numerous potentials for industrial practitioners to implement personalised and refined OM services.

The remainder of this paper is organised as follows. [Section 2](https://www.sciencedirect.com/science/article/pii/S0736584522001673#sec0002) reviews the relevant literature. [Section 3](https://www.sciencedirect.com/science/article/pii/S0736584522001673#sec0006) and [Section 4](https://www.sciencedirect.com/science/article/pii/S0736584522001673#sec0007) outline the overall framework and solution of POMA-CP based on equipment portrait, respectively. In [Section 5](https://www.sciencedirect.com/science/article/pii/S0736584522001673#sec0020), an application scenario is presented to illustrate the feasibility and effectiveness of the proposed approach. The conclusions, limitations, and future works are summarised in [Section 6](https://www.sciencedirect.com/science/article/pii/S0736584522001673#sec0025).

## 2\. Literature review

This section briefly reviews related studies in two aspects: (1) user portrait and equipment portrait, and (2) PSS-based product [OM](https://www.sciencedirect.com/topics/engineering/operation-and-maintenance "Learn more about OM from ScienceDirect's AI-generated Topic Pages") service. The application of PSS mode creates the capability for [OEMs](https://www.sciencedirect.com/topics/engineering/original-equipment-manufacturer "Learn more about OEMs from ScienceDirect's AI-generated Topic Pages") to better control and manage their products, which in turn makes it possible to access and acquire the whole [lifecycle data](https://www.sciencedirect.com/topics/engineering/lifecycle-data "Learn more about lifecycle data from ScienceDirect's AI-generated Topic Pages") of PSS delivery processes \[[6](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0006),[35](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0034)\]. These data can be analysed to provide sufficient feature labels set for constructing of more accurate and refined equipment portrait. Meanwhile, a promising application of equipment portrait is the use of its feature labels for specific scenarios to improve the efficiency and accuracy of OM services decision-making. Therefore, the PSS mode and portrait technology (user portrait or equipment portrait) are two major pillars to exploit the potential of lifecycle data and to carry out the personalised and refined product OM services. Knowledge gaps identified from the review are summarised at the end of [Section 2](https://www.sciencedirect.com/science/article/pii/S0736584522001673#sec0002).

### 2.1. User portrait and equipment portrait

The most cited definition of user portrait was introduced by Cooper \[[36](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0035)\], who believed that user portrait was a virtual representation of real users. Meanwhile, the user portrait can be divided into different types according to different behaviours and motivations. As a result, the common features of different users can be extracted and described by some labels. The user portrait is based on the real user. However, it is not specific to a certain person or user. Meanwhile, user portrait emphasises the research on a certain type of object with common characteristics and standards. Therefore, for user portraits, a virtual object usually corresponds to one or a class of physical entities.

With the advent of big data, the connotation of user portraits has changed. Gauch and Speretta believed that user portrait is mainly composed of weighted keywords and semantic web \[[37](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0036)\]. Teixeira et al. \[[38](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0037)\] pointed out that a user portrait is an information set of personal data extracted from massive user data and a model to describe a user's needs and preferences. The main challenges of user portrait applications in adaptive [recommender systems](https://www.sciencedirect.com/topics/computer-science/recommender-systems "Learn more about recommender systems from ScienceDirect's AI-generated Topic Pages") and intelligent e-commerce systems were investigated by Schiaffino and Amandi \[[39](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0038)\]. To improve the accuracy and performance of user portraits, an ontology-based portrait learning method was proposed \[[40](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0039)\]. The author found that the user portrait information can be enriched by integrating data mining and semantics. In the [big data environment](https://www.sciencedirect.com/topics/computer-science/big-data-environment "Learn more about big data environment from ScienceDirect's AI-generated Topic Pages"), user portrait and its privacy challenges were discussed by Hasan et al. \[[41](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0040)\]. The authors investigated a concrete example of constructing a portrait model with [big data techniques](https://www.sciencedirect.com/topics/computer-science/big-data-technique "Learn more about big data techniques from ScienceDirect's AI-generated Topic Pages") and the approach to preserving user privacy. A user behaviour portrait was developed based on the mobile phone [trajectory data](https://www.sciencedirect.com/topics/computer-science/trajectory-data "Learn more about trajectory data from ScienceDirect's AI-generated Topic Pages") to judge [airport](https://www.sciencedirect.com/topics/engineering/airfield "Learn more about airport from ScienceDirect's AI-generated Topic Pages") congestion and provide suggestions for airport users \[[42](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0041)\]. A user portrait-based [transfer learning](https://www.sciencedirect.com/topics/computer-science/transfer-learning "Learn more about transfer learning from ScienceDirect's AI-generated Topic Pages") mechanism was designed to conduct knowledge transfer amongst the cross-domain recommender systems \[[43](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0042)\]. The results showed that the accuracy of the proposed method substantially outperforms other methods. To solve the sparse data problem of e-commerce recommender systems, a diffusion-based algorithm was proposed to generate the user portrait and perform personalised and high-quality recommendations \[[44](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0043)\]. The experiments showed that the proposed algorithm outperforms the traditional collaborative [filtering algorithm](https://www.sciencedirect.com/topics/engineering/filtering-algorithm "Learn more about filtering algorithm from ScienceDirect's AI-generated Topic Pages"). An intelligent method for constructing the academic WeChat user portrait was proposed to solve the asymmetry problem between users’ precise knowledge needs and academic WeChat extensive knowledge services \[[45](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0044)\]. According to equipment portrait, a task-orientated adaptive system was designed to provide an intelligent maintenance implementation process for complex products (e.g. automobiles, aeroplanes, and machine tools) \[[25](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0024)\]. Based on the features of different ambient temperatures and different life stages of electric vehicles' batteries, a charging unit portrait model was designed to optimise the battery OM services and to minimise degradation over the idling period while still satisfying the charging energy requirement \[[30](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0029)\].

### 2.2. PSS-based product OM service

As reported by Si et al. \[[46](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0045)\] and Palmarini et al. \[[47](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0046)\], their reliability can be improved by analysing the operational data of complex products and executing relevant maintenance services. Due to the advantages of PSS for reducing product defects and improving maintenance task accuracy \[[6](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0006),[8](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0008),[48](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0047)\], the research on combining OM service and PSS is attracting more scholars’ interest.

Marchi et al. \[[49](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0048)\] investigated the advantages of PSS-based maintenance activities for enhancing competitiveness and sustainability in the steel industry. The authors also analysed the influence of PSS design decisions on product reliability and OM activities. In order to increase productivity and perform accurate OM service, a PSS-based maintenance method supported by [augmented reality](https://www.sciencedirect.com/topics/engineering/augmented-reality "Learn more about augmented reality from ScienceDirect's AI-generated Topic Pages") was developed \[[31](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0030)\]. The authors found that the proposed method can be used to calculate the remaining operating time between failures and identify the available windows of the products to perform remote maintenance. After this, an assistance application of PSS-based [preventive maintenance](https://www.sciencedirect.com/topics/engineering/preventive-maintenance "Learn more about preventive maintenance from ScienceDirect's AI-generated Topic Pages") for manufacturing equipment was introduced to improve the internal communication of Business to Business (B2B) and reduce time and cost during the maintenance service procedures \[[15](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0014)\]. An adaptive and PSS-based maintenance grouping strategy for complex multi-component systems was proposed to implement [predictive maintenance](https://www.sciencedirect.com/topics/engineering/predictive-maintenance "Learn more about predictive maintenance from ScienceDirect's AI-generated Topic Pages") planning \[[50](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0049)\]. The author claimed that the proposed strategy could provide long-term dynamic planning for PSS providers. Within the [Internet of Things](https://www.sciencedirect.com/topics/engineering/internet-of-things "Learn more about Internet of Things from ScienceDirect's AI-generated Topic Pages") (IoT) environment, how industrial practitioners can integrate real-time data and predictive analytics algorithms to manage maintenance policies dynamically and to realise PSS mode were explored by March and Scudder \[[51](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0050)\]. In the use-orientated PSS, a heuristic replacement policy and inventory control approach for condition-based product maintenance was formulated to improve the service provider's profitability \[[52](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0051)\]. A single case of engineering services for the maintenance and modification operations in offshore petroleum [logistics operations](https://www.sciencedirect.com/topics/engineering/logistics-operation "Learn more about logistics operations from ScienceDirect's AI-generated Topic Pages") was studied to reveal the dynamics of building network capabilities in a consistent network structure, and to investigate the problem of manage economisation in service provision and use processes \[[53](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0052)\]. Based on the PSS mode, an opportunistic maintenance policy was developed to handle the challenges of multi-procedure capacity for serial-parallel leased systems and maximise the leasing profits \[[21](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0020),[54](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0053)\]. The authors suggest that the proposed policy can schedule maintenance intervals and can the maintenance decisions dynamically. A [joint](https://www.sciencedirect.com/topics/engineering/joints-structural-components "Learn more about joint from ScienceDirect's AI-generated Topic Pages") optimisation model of maintenance and spare parts ordering policy was proposed for the use-orientated PSS to address the hardware failures and replacement spare parts problems \[[55](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0054)\]. The author demonstrated that the proposed policy could maximise system effectiveness by simultaneously considering the availability of systems, spares and repair engineers. A dual-perspective and data-based decision-making framework followed the PSS mode was developed and tested to manage the maintenance service delivery process of manufacturing companies \[[56](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0055)\]. Through a semi-structured interview, the authors found that the proposed framework can be used by manufacturing companies to explore the data generated in maintenance and use phase, and to improve their knowledge of machines and service processes. A framework of IoT enabled circular production and maintenance for automotive parts was outlined to facilitate organisation to pursue the integration of environmentally aware solutions in their production systems \[[57](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0056)\]. The proposed framework set out an agenda for digital maintenance practice within the CE and the utilisation of [Industry 4.0](https://www.sciencedirect.com/topics/computer-science/industry-4-0 "Learn more about Industry 4.0 from ScienceDirect's AI-generated Topic Pages") technologies.

### 2.3. Knowledge gaps

Although significant progress has been made in the two research dimensions mentioned above (as shown in [Table 1](https://www.sciencedirect.com/science/article/pii/S0736584522001673#tbl0001)), some gaps still need to be filled.

-   n
    
    In terms of user portrait or equipment portrait, most existing studies mainly focused on the field of computer science, information science, e-commerce or recommender system, etc. The research in the industrial field for complex products was still in its infancy and seldom investigated.
    
-   n
    
    In respect of product OM service, most existing studies combined either equipment portrait and maintenance service or aligned PSS with product and system maintenance policy planning. Little effort was put into the integrated application of equipment portrait and PSS.
    

Table 1. Classification and comparison of related studies.

| Aspects | Application field | Literature and subjects | Gaps | Challenges and coverage of this paper |
| --- | --- | --- | --- | --- |
| User portrait and equipment portrait | • Computer science  
• Information science  
• Recommender system  
• E-commerce  
• … | Software product development and maintenance \[[24](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0023)\]; privacy protection for recommender systems in big data environment \[[37](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0036),[41](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0040)\]; personalised information service for university users \[[38](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0037)\]; challenges of the user profile in recommender and e-commerce systems \[[39](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0038)\]; ontology-based profile learning and improvement method \[[40](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0039)\]; airport congestion analysing based on using mobile phone trajectory data \[[42](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0041)\]; cross-domain knowledge transfer and recommender mechanism \[[43](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0042),[44](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0043)\]; academic resource needs and preference habits analysing \[[45](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0044)\]; information retrieval pattern analysing of digital library users \[[58](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0057)\]; enterprise public opinion recognising based on high-risk users \[[59](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0058)\]; user profile-based network crime fast detection method \[[60](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0059)\] | Most existing studies mainly focused on the field of computer science, information science, e-commerce or recommender system, etc. The research of user portrait or equipment portrait in the industrial field was seldom investigated. | How to develop a prescriptive procedure of equipment portrait for the industrial field and enrich portrait technology application scenarios in the personalised product OM service? |
| • Industry | Machine tool OM \[[25](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0024)\]; water supply infrastructure OM \[[26](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0025)\]; electric vehicle battery OM \[[30](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0029)\] |
| PSS-based product OM service | • Industry | Active preventive maintenance based on PSS and lifecycle big data \[[6](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0006),[8](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0008)\]; connecting OM service and PSS to enhance machine availability \[[14](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0013)\]; unscheduled OM service method for manufacturing equipment \[[15](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0014)\]; PSS-based maintenance planning system for knowledge sharing \[[16](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0015)\]; PSS-based maintenance grouping strategy for manufacturing system \[[17](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0016)\]; process-based OM service knowledge management mechanism \[[18](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0017)\]; lease-orientated opportunistic maintenance for production system \[[21](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0020),[54](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0053)\]; PSS-based augmented reality maintenance for shop floor \[[31](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0030)\]; maintenance strategy for complex multi-component systems \[[50](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0049)\]; inventory control approach for condition-based maintenance \[[52](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0051)\]; economisation of maintenance and modification operations \[[53](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0052)\]; maintenance and spares ordering policy for use-orientated PSS \[[55](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0054)\];  
a data-driven decision framework of maintenance service delivery process \[[56](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0055)\]; IoT-enabled circular production and maintenance framework \[[57](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0056)\] | Most existing studies combined either equipment portrait and maintenance service or aligned PSS with product and system maintenance policy planning. Little effort was put into the integration application of equipment portrait and PSS. | By means of the PSS delivery processes data, how to establish a precise portrait model to implement performance-based proactive and refined OM service for complex products? |

## 3\. An overview framework of the POMA-CP based on equipment portrait

This section outlines a novel approach for personalised product OM service. The objective of the proposed approach is to better integrate the PSS delivery processes with equipment portrait to achieve personalised and refined services for complex products.

The overall framework of the proposed approach is designed and depicted in [Fig. 1](https://www.sciencedirect.com/science/article/pii/S0736584522001673#fig0001). Under this framework, the equipment portrait model can be dynamically changed and updated through the shared data and knowledge amongst different [lifecycle stages](https://www.sciencedirect.com/topics/computer-science/lifecycle-stage "Learn more about lifecycle stages from ScienceDirect's AI-generated Topic Pages") of PSS delivery processes. Consequently, personalised, refined and accurate OM service can be achieved. The proposed framework includes four components, namely (1) lifecycle data acquisition and value-added processing, (2) OM knowledge mining and service case management, (3) equipment portrait modelling and OM scheme generation, and (4) application services of personalised OM for complex products.

![Fig 1](https://ars.els-cdn.com/content/image/1-s2.0-S0736584522001673-gr1.jpg)

1.  [Download : Download high-res image (1MB)](https://ars.els-cdn.com/content/image/1-s2.0-S0736584522001673-gr1_lrg.jpg "Download high-res image (1MB)")
2.  [Download : Download full-size image](https://ars.els-cdn.com/content/image/1-s2.0-S0736584522001673-gr1.jpg "Download full-size image")

Fig. 1. The overall framework of the POMA-CP based on equipment portrait.

Component 1 is responsible for acquiring product lifecycle data by emerging perceptive technology and achieving data value-added by bill of material (BOM) mapping. First, the [IoT](https://www.sciencedirect.com/topics/engineering/internet-of-things "Learn more about IoT from ScienceDirect's AI-generated Topic Pages") devices and [smart sensors](https://www.sciencedirect.com/topics/engineering/smart-sensors "Learn more about smart sensors from ScienceDirect's AI-generated Topic Pages") are configured for complex products and their key components at the beginning of life to improve the self-sensing ability of products in the middle of life and the end of life \[[61](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib70)\]. For example, the real-time position and operational status data of products can be obtained through the configured GPS and smart sensors in the operation stage. Furthermore, the configured [radio frequency identification](https://www.sciencedirect.com/topics/engineering/radio-frequency-identification "Learn more about radio frequency identification from ScienceDirect's AI-generated Topic Pages") (RFID) tags and Auto-ID devices can be used to quickly identify the maintenance objects’ information in the maintenance stage. Second, various BOMs can be established based on the data of different lifecycle stages, including engineering BOM, manufacturing BOM, process planning BOM, etc. Similarly, a service BOM of the OM stage can be built to organise the massive OM data and provide value-added data for the OM service decision-making. In this paper, a composite BOM structure (consisting of neutral BOM, instance BOM, maintenance task BOM, and collectively called xBOM) is designed to establish the service BOM. Finally, an xBOM mapping technology can be developed to ensure OM service information's consistency and traceability. Thereby, reliable and holistic data assets can be provided for the personalised and refined product OM services.

Generally, experience-based maintenance service decisions will lead to lower service efficiency and customer satisfaction. Therefore, it is necessary to mine the OM service knowledge from the historical OM data to provide powerful knowledge support for the personalised OM services by effective [knowledge management](https://www.sciencedirect.com/topics/computer-science/knowledge-management "Learn more about knowledge management from ScienceDirect's AI-generated Topic Pages") and reuse (Component 2 in [Fig. 1](https://www.sciencedirect.com/science/article/pii/S0736584522001673#fig0001)). First, the information or knowledge of product fault category (e.g., fault position, fault cause, fault occurrence time) should be analysed and mined to establish the fault tree model of complex products. Second, according to the mined fault [category knowledge](https://www.sciencedirect.com/topics/computer-science/knowledge-category "Learn more about category knowledge from ScienceDirect's AI-generated Topic Pages") and the case-based reasoning technology, the multi-level OM service case libraries can be constructed to organise the maintenance domain knowledge effectively. For example, an OM service case library can be divided into several sub-case libraries based on the different fault categories. Then, the fault feature keywords can be mapped to a specific fault category. These case libraries can provide valuable knowledge support for the following cases’ active pushing processes while reducing case search time and improving case matching efficiency. Third, by extracting fault symptom keywords and establishing a fault-feature [vector space model](https://www.sciencedirect.com/topics/computer-science/vector-space-models "Learn more about vector space model from ScienceDirect's AI-generated Topic Pages"), the characteristics of OM service cases can be expressed to manage the OM service knowledge. As a result, knowledge reuse for personalised OM services can be realised.

To carry out the personalised and refined OM services of complex products, equipment portraits of different operating conditions and environments should be depicted accurately (Component 3 in [Fig. 1](https://www.sciencedirect.com/science/article/pii/S0736584522001673#fig0001)). Generally, the equipment portrait is made up of a label set. Therefore, the label library of OM services should be first constructed. Based on the features and requirements of complex products’ OM services, a three dimensions label library (including behaviour, attribute and fault feature labels) is designed in this paper. Second, considering the dynamics of fault features, an equipment portrait model consisting of multiple feature items (each feature item represents a fault category) and weights can be established using the space vector. Each feature is divided into attribute, behaviour, and fault symptom. As a result, two types of portrait models (i.e., attribute and behaviour feature model and fault symptom feature model) can be constructed. Besides, different model update mechanisms should be developed to update equipment portrait dynamically and periodically. Third, the similarity between source cases (in the established sub-case library) and target cases can be calculated to obtain the best similar cases, which can be further adjusted and optimised to match and generate the final OM scheme for the target products. Meanwhile, the new case should be updated in the original case library.

Application services of personalised OM are responsible for encapsulating multi-stage OM service business, including intelligent equipment monitoring, personalised OM service scheme, refined OM service management, and component defect improvement (Component 4 in [Fig. 1](https://www.sciencedirect.com/science/article/pii/S0736584522001673#fig0001)). For example, by integrating the historical lifecycle data and the real-time operational status data of complex products, portrait models of the key components can be established and updated to monitor their fault features intelligently. By combining the fault features of individual products and the recommended best similar case in the multi-level OM service library, the personalised OM service scheme for different products can be developed. The refined OM service management can realise the control and management of personnel assignment, [task scheduling](https://www.sciencedirect.com/topics/computer-science/task-scheduling "Learn more about task scheduling from ScienceDirect's AI-generated Topic Pages"), spare parts supply, etc. According to the fault features and rules reflected by the portrait model in the OM processes, the weaknesses and defects information of current products can be identified. The information can support improving and updating the next generation of product design.

The logic and relation of the four components in [Fig. 1](https://www.sciencedirect.com/science/article/pii/S0736584522001673#fig0001) are described as follows. Component 1 can provide holistic and consistent data for Component 2 through emerging perceptive technology and xBOM mapping technology. As a result, effective knowledge mining, management and reuse in Component 2 can be achieved. Meanwhile, the real-time OM data of Component 1 provides OEMs with the opportunity to update the label library and equipment portrait dynamically and periodically. Therefore, the accuracy and effectiveness of the pushed OM service cases can be enhanced. In addition, the OM case library and knowledge reusing in Component 2 can provide knowledge support for constructing of label library and equipment portrait model in Component 3, which in turn improves the quality and reliability of generated OM scheme. Through Component 2 and Component 3, various application services related to personalised OM in Component 4 can be carried out.

## 4\. The solution of POMA-CP based on equipment portrait

According to the proposed framework, the key technologies (mainly involving Component 2 and Component 3 in [Fig. 1](https://www.sciencedirect.com/science/article/pii/S0736584522001673#fig0001)) are introduced in this section, and as a solution of POMA-CP based on equipment portrait. These technologies include case-based reasoning OM knowledge management and reuse, equipment portrait modelling, and active OM service case-pushing.

### 4.1. Case-based reasoning OM knowledge management and reuse

#### 4.1.1. Multi-level OM service case library establishing

Due to the powerful logical relation processing ability of failure mode & effect analysis (FMEA), it is widely used in fault diagnosis (especially for complex products with thousands of components) \[[62](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0060)\]. Generally, the FMEA for all complex product components is unnecessary. Therefore, the fault tree models of the critical components are established to conduct the FMEA in practice. A complete fault tree consists of top events, intermediate events and basic events. Due to the limited space, please refer to \[[63](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0061),[64](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0062)\] for more information about the main steps and specific methods for executing the FMEA.

The fault information and knowledge of complex products or critical components can be analysed and extracted based on the FMEA. To improve the efficiency of knowledge managing and reusing, a method for establishing the multi-level OM service case library based on the [FMEA and](https://www.sciencedirect.com/topics/engineering/failure-mode-and-effect-analysis "Learn more about FMEA and from ScienceDirect's AI-generated Topic Pages") the extracted fault knowledge is developed in this paper (as seen in [Fig. 2](https://www.sciencedirect.com/science/article/pii/S0736584522001673#fig0002)).

![Fig 2](https://ars.els-cdn.com/content/image/1-s2.0-S0736584522001673-gr2.jpg)

1.  [Download : Download high-res image (468KB)](https://ars.els-cdn.com/content/image/1-s2.0-S0736584522001673-gr2_lrg.jpg "Download high-res image (468KB)")
2.  [Download : Download full-size image](https://ars.els-cdn.com/content/image/1-s2.0-S0736584522001673-gr2.jpg "Download full-size image")

Fig. 2. The structure for multi-level OM service case library of complex products.

First, the complex products are divided into _Z_ critical component systems (represented by _[CCS](https://www.sciencedirect.com/topics/engineering/carbon-capture-and-storage "Learn more about CCS from ScienceDirect's AI-generated Topic Pages")<sub>z</sub>_) according to their structure and composition. Second, for the _z_th _CCS<sub>z</sub>_, it is divided into _n_ sub-case library (represented by _SCL<sub>z,i</sub>_, ) according to its _n_ fault category (represented by _FC<sub>z,i</sub>_). Third, in the _i_th _SCL<sub>z,i</sub>_, a number of fault symptoms are included. Assume that a total of _m<sub>i</sub>_ fault symptom keywords (represented by _FSK<sub>z,i,j</sub>_, ) are extracted from the fault symptom. Each OM service case (represented by _CASE<sub>z,i,k</sub>_, ) in the _SCL<sub>z,i</sub>_ corresponds to a fault feature space vector that is composed of multiple _FSK_, and contains the essential attribute of product fault and corresponding maintenance scheme (represented by _MS<sub>z,i,k</sub>_). The method of _FSK_ extracting and _CASE_ representation will be described in the following section in detail.

#### 4.1.2. Case-based reasoning OM knowledge reusing

In this paper, the _CASE_ is regarded as a basic unit of OM knowledge, and each _CASE_ includes some fault information such as fault locations, categories, symptoms and maintenance measures. Therefore, the accuracy of the _CASE_ information expression will greatly affect the efficiency of OM knowledge reuse. Here, the _CASE_ expression refers to a structured description of the information in the historical _CASE_, which is a method of organising the case information according to a specific format. The _CASE_ for complex products can be defined as a four-tuple, as shown below:(1)where, describes the basic information of the _CASE_ for the complex product _CCS_, including case number, fault time, fault location, and fault [symptom description](https://www.sciencedirect.com/topics/computer-science/symptom-description "Learn more about symptom description from ScienceDirect's AI-generated Topic Pages") text, etc. (_S, E_) is the feature set of the _CASE_, where _S_ is the fault symptom feature vector of _CASE_ and represented as , including the _FSK_ () of _CASE_ and its corresponding weight . _E_ is the attribute label information and behaviour label information of _CASE_, including fault location, operating environment, operating parameters, and represented as . Finally, _Q_ is the fault conclusion, including the information on OM service solutions and evaluation results.

The method for establishing attribute labels and behaviour labels is illustrated in [Section 4.2](https://www.sciencedirect.com/science/article/pii/S0736584522001673#sec0013). The following content of this section will introduce the extraction method of _FSK_ and the [construction processes](https://www.sciencedirect.com/topics/engineering/construction-process "Learn more about construction processes from ScienceDirect's AI-generated Topic Pages") of the fault symptom feature vector of _CASE_.

##### 4.1.2.1. Fault symptom keywords extracting

To quantitatively express the fault symptom feature vector of the _CASE_ text, the text segmentation operation should be performed on the fault record text to obtain the _FSK_. As the best word segmentation module of Python, the _Jieba_ algorithm is widely used in Chinese text segmentation due to its high accuracy and supporting customised dictionaries and multiple modes \[[65](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0063)\].

However, for a specific complex industrial product, the existing text segmentation tool is difficult to deal with the professional terms effectively. Therefore, it is necessary to supplement and customise the professional dictionary of _CASE_ before performing the text segmentation operation and then carry out the following operations:

-   1)
    
    Based on the constructed professional dictionary of _CASE_, the _Jieba_ tokeniser is used to carry out the text segmentation operation, and the forward maximum [matching algorithm](https://www.sciencedirect.com/topics/engineering/matching-algorithm "Learn more about matching algorithm from ScienceDirect's AI-generated Topic Pages") is adopted to capture the string as long as possible.
    
-   2)
    
    Based on the text segmentation operation of _CASE_, the fault feature words set will be obtained by setting a stop word table and eliminating the function words and prepositions in fault record sentences.
    
-   3)
    
    Extract the feature words used to express the OM service domain knowledge of complex products to construct the _FSK_ lexicon. Then, the Chi-square test is used to judge the correlation between fault categories and words in the _FSK_ lexicon. Finally, the words that have a high correlation with product OM service are extracted as the _FSK_. The calculation process of correlation is described in detail as follows:
    

For a specific _CCS<sub>z</sub>_, supposing it contains _n FC<sub>z,i</sub>_, and the _CCS<sub>z</sub>_ has _N CASE_. Specifically, for a fault feature word _T_, regarding whether it contains _T_ and whether it belongs to _FC<sub>z,i</sub>_ as the judging conditions, and four values A, B, C and D are obtained (as shown in [Table 2](https://www.sciencedirect.com/science/article/pii/S0736584522001673#tbl0002)).

Table 2. The relationships between categories and attributes.

| Feature selection | Belong to _FC<sub>z,i</sub>_ | Not belong to _FC<sub>z,i</sub>_ | Event name |
| --- | --- | --- | --- |
| Contain _T_ | A | B | _A_ + _B_ |
| Not contain _T_ | C | D | C + D |
| Total | A + C | _B_ + D | _A_ + B + C + D |

Calculating the theoretical value _E<sub>A</sub>_ and difference value _D<sub>A</sub>_ of A by the following formula:(2)(3)(4)

The method for calculating the theoretical value and difference value for B, C and D is similar to A. The specific methods are not repeated here. Then, the chi-square value between fault feature word _T_ and _FC<sub>z,i</sub>_ is calculated by using the original hypothesis that _T_ is not correlated with _FC<sub>z,i</sub>_:(5)

At the same time, the chi-square value of the fault feature word _T_ relative to the whole _CASE_ library of the _CCS<sub>z</sub>_ is calculated:(6)where, represents the probability that the case in the _CASE_ library of the _CCS<sub>z</sub>_ belongs to the _FC<sub>z,i</sub>_. The greater the value , the larger the error for the original hypothesis. That is, the fault feature word _T_ has a strong correlation with the _FC<sub>z,i</sub>_. Subsequently, calculating the chi-square values of all fault feature words according to the abovementioned method and arranging them from large to small. Meanwhile, a certain number of fault feature words are screened according to a preset threshold value. As a result, the _FSK_ lexicon can be established.

##### 4.1.2.2. Fault symptom feature vector constructing of the _case_

The _i_th _SCL<sub>z,i</sub>_ corresponds to the _i_th _FC<sub>z,i</sub>_ of the _z_th _CCS<sub>z</sub>_ (as seen in [Fig. 2](https://www.sciencedirect.com/science/article/pii/S0736584522001673#fig0002)), and the _k_th _CASE<sub>z,i,k</sub>_ is analysed. Therefore, _R_ fault symptom keywords (i.e., ) contained in the case text are first extracted, and then the [vector space model](https://www.sciencedirect.com/topics/computer-science/vector-space-models "Learn more about vector space model from ScienceDirect's AI-generated Topic Pages") of the fault symptom feature of the _CASE_ is established. Here, the case text is regarded as a vector space composed of a group of [orthogonal vectors](https://www.sciencedirect.com/topics/engineering/orthogonal-vector "Learn more about orthogonal vectors from ScienceDirect's AI-generated Topic Pages"). The fault symptom feature vector of the _CASE_ is expressed as , () is the _FSK_, is the weight of each _FSK_.

The weight can be calculated by the term frequency-inverse document frequency (TF-IDF) algorithm. The TF-IDF is a feature weight algorithm based on term frequency, which is used to evaluate the importance of a feature word to a particular text in a file set \[[66](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0064),[67](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0065)\]. TF is the word frequency of feature words in a particular text, and IDF measures the importance of a feature word. Therefore, the weight of is expressed as a function of and . The is calculated by:(7)where, refers to the times of occurred in a certain _CASE_, and is the total number of occurred in the _CASE_.

Meanwhile, the is calculated by:(8)where, is the total number of all _CASE_ in the _FC<sub>z,i</sub>_, is the number of _CASE_ that contain in the _FC<sub>z,i</sub>_.

The weight of is calculated by:(9)

The greater the value (i.e., TF-IDF value) of , the more important it is to the _CASE_. Based on the obtained weight of each , the feature selection weight of (namely the weight threshold of TF-IDF) is defined and determined by the following equation:(10)

If , the of the _CASE_ is retained; if , the of the _CASE_ is excluded. Finally, the retained is used to construct the fault symptom feature vector of the _CASE_.

Furthermore, the weight can be further normalised by the following formula:(11)

### 4.2. Equipment portrait modelling and OM service case active pushing

#### 4.2.1. Label library constructing and equipment portrait modelling

A proper label library is significant to establishing an equipment portrait model. Generally, a label is a tuple containing product attributes and values. It can be described as _Label_\=<_Name: W_\>, where _name_ indicates the label name, and _W_ is the weight. The type and value of weight are determined by label attributes, including numerical type, interval [data type](https://www.sciencedirect.com/topics/computer-science/data-type "Learn more about data type from ScienceDirect's AI-generated Topic Pages"), text type, etc.).

##### 4.2.1.1. Label library designing for complex products

In view of the characteristics and OM service requirements of complex industrial products, a three dimensions label library (including attribute label, behaviour label and fault symptom label) is designed in this paper. They are briefly introduced as follows:

The attribute label represents the inherent static attributes of complex products, such as equipment type, manufacturer, rated power, and other information. This information will not change dynamically over time and can be obtained directly from the product database.

The behaviour label consists of the operational status information of complex products, which may change dynamically with time (e.g., operational time, average failure rate, maintenance times, operational environment). Such labels can be obtained by statistical or logical calculation.

The fault symptom label is similar to the _FSK_ in [Section 4.1](https://www.sciencedirect.com/science/article/pii/S0736584522001673#sec0008). Such labels may change dynamically over time and can be obtained by analysing and mining complex products’ operational performance record text. For example, the [operational load](https://www.sciencedirect.com/topics/engineering/operational-load "Learn more about operational load from ScienceDirect's AI-generated Topic Pages") is too high, abnormal wear in the bearing shell, bearing temperature is too high, etc.

##### 4.2.1.2. Portrait modelling for complex products

Based on the label library, an equipment portrait model of complex products can be developed. For the _CCS_ of complex products, the equipment portrait model _M<sub>u</sub>_ can be defined as follows:(12)where (_F<sub>i</sub>, E<sub>i</sub>_) represents the fault category feature vector that corresponds to _FC<sub>i</sub>_ in the _CCS_, each (_F<sub>i</sub>, E<sub>i</sub>_) is associated with a weight _ω<sub>i</sub>_. The weight _ω<sub>i</sub>_ refers to the proportion of the occurred times of _F<sub>i</sub>_ amongst all fault categories in a fixed time window. In particular, only the feature vectors whose weights are greater than the threshold _ω<sub>threshold</sub>_ can appear in the equipment portrait model _M<sub>u</sub>_.

The fault category feature vector (_F<sub>i</sub>, E<sub>i</sub>_) mainly contains:

-   (1)
    
    **Fault symptom feature model _F<sub>i</sub>_**. Considering the name of labels in the fault symptom feature model are dynamical changing, it can be described as:(13)
    

where () is the th fault symptom label; is the weight, which indicates the importance of the fault feature; is the time when the label is added to _F<sub>i</sub>_; is the time when the label is last updated in _F<sub>i</sub>_. The weight consists of the local weight and global weight of the label, which is calculated by the following formula:(14)

-   1)
    
    Local weight of the label
    

The value of the local weight is determined by the occurred times of fault symptom label in the . To avoid , the local label is redefined as:(15)where indicates the occurred times of label in in the historical OM records.

-   1)
    
    Global weight of the label
    

The local weight cannot be used to measure the accuracy of different labels in identifying various fault categories. Therefore, the concept of [conditional entropy](https://www.sciencedirect.com/topics/engineering/conditional-entropy "Learn more about conditional entropy from ScienceDirect's AI-generated Topic Pages") is introduced to define the global weight of the label \[[68](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0066)\], which is represented by . First, the fault categories set is seen as the information sources which conform to some probability distribution, and the _FC_ and the label are regarded as two [random variables](https://www.sciencedirect.com/topics/engineering/random-variable-xi "Learn more about random variables from ScienceDirect's AI-generated Topic Pages"). Second, the importance of a label to a certain _FC_ is determined by the gain between the information entropy of the fault categories set and the conditional entropy of the label \[[69](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0067)\]. Finally, the conditional entropy of the _FC,_ which corresponds to the label, can be calculated by the following formula:(16)where indicates the probability of when the label appears, when the label is determined, . As a result, the above equation can be expressed as:(17)where indicates the number of _FC_ in the fault categories set; indicates the times of in ; indicates the times of in the whole fault categories set.

-   (1)
    
    **Attribute and behaviour feature model _E<sub>i</sub>_**. Considering the name of labels for attribute and behaviour feature model is fixed, the occurred times of label do not need to be considered when establishing the . Therefore, the attribute and behaviour feature model can be described as:(18)
    

where () represents the attribute label or behaviour label. It contains detailed information related to the _FC_ and other essential information (such as manufacturer, component number, operational status, [service duration](https://www.sciencedirect.com/topics/computer-science/service-duration "Learn more about service duration from ScienceDirect's AI-generated Topic Pages"), operational load, etc.); is the label content.

When a label of an abnormal event appears in the OM service abnormal event log text set, it is important to map it to the corresponding FC accurately, and then the personalised and refined OM services can be realised. To achieve this objective, the [conditional probability](https://www.sciencedirect.com/topics/engineering/conditional-probability "Learn more about conditional probability from ScienceDirect's AI-generated Topic Pages") of the abnormal event label belonging to each _FC_ should be first calculated. Then the _FC_ of the abnormal event label is divided into the categories with the highest conditional probability value. The main steps for calculating the conditional probability of the abnormal event label are as follows:

-   1)
    
    Given an abnormal event log text as a sample set, where the _FC_ is known. Calculating the conditional probability of each abnormal event label in each _FC_:(19)
    
-   2)
    
    Assuming that the abnormal event labels of the sample set are independent of each other, the probability of an abnormal event log text with a label belonging to the FCi is calculated by the following formula:(20)P(FCi|attr)\=P(attr|FCi)P(FCi)P(attr)
    
-   3)
    
    For all fault categories, the P(attr) can be regarded as a constant, so P(attr) can be removed and P(attr|FCi)P(FCi) can be calculated separately, namely:(21)P(attr|FCi)P(FCi)\=P(attr1|FCi),P(attr2|FCi),...,P(attrm|FCi)P(FCi)\=P(FCi)∏j\=1mP(attrj|FCi)
    
-   4)
    
    Therefore, the _FC_ of the abnormal event log text is determined as:(22)FC\=argmaxP(FCi)∏j\=1mP(attrj|FCi)
    

The above method regards the weight of all labels as equal. However, various labels play different roles in different [text classification](https://www.sciencedirect.com/topics/computer-science/text-classification "Learn more about text classification from ScienceDirect's AI-generated Topic Pages") application scenarios. To improve the text classification performance, the TF-IDF algorithm in [Section 4.1.2](https://www.sciencedirect.com/science/article/pii/S0736584522001673#sec0010) is used to calculate the weight of abnormal event text labels for the _CASE_ and carry out the mapping between abnormal event labels and fault categories.

Assume that the abnormal event log text of a _CASE_ has a label attr\={attr1,attr2,...,attrm}, the term frequency of attrj is calculated by TF(attrj)\=nj∑i\=1mni, nj refers to the occurred times of attrj in the abnormal event log text, and ∑i\=1mni is the total number of the attrj occurred in the _CASE_. Meanwhile, the [inverse document frequency](https://www.sciencedirect.com/topics/computer-science/inverse-document-frequency "Learn more about inverse document frequency from ScienceDirect's AI-generated Topic Pages") of attrj in the abnormal event relative to the FCi is calculated by IDF(attrj,FCi)\=logNiNi,j+1, Ni is the number of cases in the _CASE_ library of the FCi, and Ni,j is the number of cases in the _CASE_ library where FCi contains label attrj.

For the abnormal event log text that contains the label attr\={attr1,attr2,...,attrm}, the weight ωi,j of label attrj relative to the FCi is calculated by:(23)ωi,j\=TF(attrj)\*IDF(attrj,FCi)

Finally, the _FC_ of the abnormal event log text is identified as:(24)FC\=argmaxP(FCi)∏j\=1mP(attrj|FCi)ωi,j

#### 4.2.2. OM service case active pushing and scheme decision-making

Due to the dynamic changing of behaviour label and fault symptom label in the equipment portrait model, real-time monitoring and tracing of the CCS that frequently occurred faults can be achieved. As a result, the reuse of historical OM knowledge can be realised. Based on the dynamic monitoring of _CCS_ and efficient reusing of OM knowledge, an active pushing mechanism for the _CASE_ is proposed. The processes of active pushing of the _CASE_ for complex products are shown in [Fig. 3](https://www.sciencedirect.com/science/article/pii/S0736584522001673#fig0003). For each feature item of the equipment portrait model, the similarity between the sub-case library and the target case should be calculated to obtain the best similar cases. Then, the best similar cases are adjusted according to the target case's requirements and the OM service scheme is generated applicable to the target product. The local similarity and global similarity of _CASE_ are involved.

![Fig 3](https://ars.els-cdn.com/content/image/1-s2.0-S0736584522001673-gr3.jpg)

1.  [Download : Download high-res image (542KB)](https://ars.els-cdn.com/content/image/1-s2.0-S0736584522001673-gr3_lrg.jpg "Download high-res image (542KB)")
2.  [Download : Download full-size image](https://ars.els-cdn.com/content/image/1-s2.0-S0736584522001673-gr3.jpg "Download full-size image")

Fig. 3. The active pushing processes of complex products’ OM service cases.

##### 4.2.2.1. The local similarity calculation of OM service case

Assuming that the fault category feature vector of _FC<sub>i</sub>_ in equipment portrait model _M<sub>u</sub>_ is (_F<sub>i</sub>, E<sub>i</sub>_). Let CASEFCi\={CASEFCi,1,CASEFCi,2,....} represents the _SCL_ of _FC<sub>i</sub>_, and CASEFCi,k represents the kth _CASE_ of the _SCL_. Usually, the types of label values for the equipment portrait model mainly include symbols, numerical values, interval numbers and texts. Therefore, to achieve the active pushing of _CASE_ and the refined OM services, it is necessary to calculate the local similarity for different label values. Usually, the label value types for model _E<sub>i</sub>_ are mainly symbols, numerical values and interval numbers, while the label value types for model _F<sub>i</sub>_ are mainly texts.

For a label attrj, let LS(Eij,CASEFCi,kj) represent the local similarity between model Ei and CASEFCi,k, WEj and WFCj represents the values of Ei and CASEFCi,k, respectively. Meanwhile, let LS(Fij,CASEFCi,kj)represent the local similarity between model Fi and CASEFCi,k. The calculation methods of local similarity for different label value types are introduced as follows.

-   (1)
    
    Symbol
    

Symbol labels usually have a fixed value set, enumerating all possible values for these labels. And each value is represented by an explicit symbol. For example, the value of the label ‘_CNC machine_’ can be _JV_<sub>1</sub>, _JV_<sub>2</sub>, _JV_<sub>3</sub>, _JV_<sub>5</sub>, etc. Then, the local similarity of the symbol label is calculated by:(25)LS(Eij,CASEFCi,kj)\={1WEj\=WFCj0WEj≠WFCj

-   (1)
    
    Numerical value
    

Let max(CASEFCij) and min(CASEFCij) represent the maximum value and minimum value of label attrj in the _SCL_ that correspond to the FCi, respectively. Therefore, the local similarity of the numerical value label is calculated by:(26)LS(Eij,CASEFCi,kj)\=1−|WEj−WFCj|max(CASEFCij)−min(CASEFCij)

-   (1)
    
    Interval number
    

The upper and lower bounds of the interval are represented by Lj and Hj respectively, and label attrj∈\[Lj,Hj\]. Therefore, the local similarity of the interval number label is calculated by:(27)LS(Eij,CASEFCi,kj)\=1−|WEj−LjHj−Lj−WFCj−LjHj−Lj|

-   (1)
    
    Text
    

Text similarity is mainly reflected in the fault symptom feature model Fi. That is, if the label is an evaluation label, the label name is a text type, and the value is the weight of each label. Given Fi and a case S\={(attr1:η1),...,(attrm:ηm)} in the CASEFCi,k. Only the common labels of Fi and S are considered. Thereby, Fi\={(attr1F:η1F),...,(attrnumFF:ηnumFF)}, S\={(attr1FC:η1FC),...,(attrnumFFC:ηnumFFC)}. Let Fij is the jth common label attrjF in Fi, and its similarity is LS(Fij,CASEFCi,kj)\=ηjF·ηjFC. As a result, the local similarity of text label is calculated by:(28)LS(Fi,CASEFCi,k)\=∑j\=1numFηjF·ηjFC

##### 4.2.2.2. The global similarity calculation of OM service case

Local similarity can only be used to measure the relationship between Fi (or Ei) of equipment portrait model _M<sub>u</sub>_ and the _CASE_. Therefore, the local similarity calculation for different label values has some limitations. For instance, it is difficult to achieve accurate pushing of the _CASE_ and thus difficult to carry out refined OM service. To overcome these limitations, it is necessary to calculate further the global similarity between the (_F<sub>i</sub>, E<sub>i</sub>_) of equipment portrait model _M<sub>u</sub>_ and _CASE_.

Considering the dynamic changing of the label in _F<sub>i</sub>_ and the missing attribute of the label in _E<sub>i</sub>_, the structure similarity should be involved while calculating the global similarity between (_F<sub>i</sub>, E<sub>i</sub>_) and CASEFCi,k. The global similarity GS(Fi,CASEFCi,k) of the fault symptom feature model _F<sub>i</sub>_ and CASEFCi,k is obtained by summing up the local similarity of the common fault symptom labels of _F<sub>i</sub>_ and CASEFCi,k, and then multiplying the structural similarity:(29)GS(Fi,CASEFCi,k)\=JS(Fi,CASEFCi,k)·LS(Fi,CASEFCi,k)\=NFi∩CASEFCi,kNFi∪CASEFCi,k·∑j\=1numFηjF·ηjFCwhere, JS(Fi,CASEFCi,k) is the structural similarity of _F<sub>i</sub>_ and CASEFCi,k; NFi∩CASEFCi,k is the number of attributes for the [intersection set](https://www.sciencedirect.com/topics/computer-science/intersection-set "Learn more about intersection set from ScienceDirect's AI-generated Topic Pages") of _F<sub>i</sub>_ and CASEFCi,k; NFi∪CASEFCi,k is the number of attributes for the union set of _F<sub>i</sub>_ and CASEFCi,k; numF is the number of common fault symptom labels.

In the same way, the global similarity GS(Ei,CASEFCi,k) between the attribute and behaviour feature model _E<sub>i</sub>_ and the CASEFCi,k is obtained by summing up the local similarity of the common attribute and behaviour labels of _E<sub>i</sub>_ and CASEFCi,k, and then multiplying the structural similarity:(30)GS(Ei,CASEFCi,k)\=JS(Ei,CASEFCi,k)·∑j\=1numELS(Eij,CASEFCi,kj)ρj∑j\=1numEρj\=NEi∩CASEFCi,kNEi∪CASEFCi,k·∑j\=1numELS(Eij,CASEFCi,kj)ρj∑j\=1numEρjwhere, JS(Ei,CASEFCi,k) is the structural similarity of _E<sub>i</sub>_ and CASEFCi,k; NEi∩CASEFCi,k is the number of attributes for the intersection set of _E<sub>i</sub>_ and CASEFCi,k; NEi∪CASEFCi,k is the number of attributes for the union set of _E<sub>i</sub>_ and CASEFCi,k; numE is the number of common attribute and behaviour labels; ρj(j\=1,2,..,num) is the weight of common label, which is determined by expert experience.

Finally, the global similarity between the (_F<sub>i</sub>, E<sub>i</sub>_) and the CASEFCi,k is calculated by the following formula:(31)GS((Fi,Ei),CASEFCi,k)\=GS(Fi,CASEFCi,k)·ρF+GS(Ei,CASEFCi,k)·ρEρF+ρEwhere, ρF is the weight of _F<sub>i</sub>_, and ρE is the weight of _E<sub>i</sub>_.

After calculating the global similarity between each (_F<sub>i</sub>, E<sub>i</sub>_) of equipment portrait model _M<sub>u</sub>_ and _CASE_ of the _SCL_, the best similar cases that correspond to these fault category feature vectors can be obtained by ranking the similarity from large to small. The higher the similarity value ranked, the more valuable the _CASE_ is for future OM service solutions. Then, a set composed of these best similar cases is formed. These best similar cases will further correspond to a set that consists of some OM service solutions. According to the real-time operational status of complex products, the solutions can be dynamically adjusted (such as deleting, adding, replacing, modifying, etc.). As a result, the final OM service scheme can be generated and actively pushed to the OM service scene. Therefore, auxiliary decision support can be provided for complex products’ personalised and refined OM services. In addition, for the _CASE_ with low similarity, it does not mean that it has no value for OM service decision-making. For example, experienced maintenance personnel can help judge and verify the usefulness of the _CASE_. As a result, the modified _CASE_ according to the abovementioned operations can be updated and added to the _SCL_ to enrich the case library, and to provide more similar cases for OM service decision-making.

## 5\. A study of the application scenario

The proposed approach is tested by an application scenario of an industrial partner company. The main objectives of the application scenario study are to test how equipment portrait can be used in the OM service of complex products and what improvements are made by the combined application of equipment portrait and PSS in personalised OM service.

In this section, an overview of the application scenario is provided. Then, the established methods of the application scenario's multi-level OM service case library are introduced. Third, the active pushing processes of OM service cases for bogie wheelsets of high-speed electric multiple units (EMUs) are elaborated. Finally, the advantages of the proposed POMA-CP based on equipment portrait within the PSS mode are analysed and discussed.

### 5.1. Overview of the application scenario

The partner company is a rail transit equipment manufacturer in China which focuses on the R&D, manufacturing, service and maintenance support of high-end railway equipment. The company has manufactured over 10,000 trains in varieties. These trains include high-speed EMUs, intercity EMUs, and urban rail vehicles used by more than 18 railway companies across China. Because the company's trains are mainly used in the passenger transport area, it is of [utmost importance](https://www.sciencedirect.com/topics/computer-science/utmost-importance "Learn more about utmost importance from ScienceDirect's AI-generated Topic Pages") for the company to prevent faults and ensure safety and reliability. In recent years, the company has developed and manufactured a series of new high-speed EMUs with a top speed of 350 km/h and 400 km/h.

Bogie is one of the important components to realise the running task of a high-speed EMU, which has a lifespan of over 20–30 years. It is responsible for guidance, [vibration reduction](https://www.sciencedirect.com/topics/engineering/vibration-reduction "Learn more about vibration reduction from ScienceDirect's AI-generated Topic Pages"), traction, braking and other tasks of the EMUs. The maintenance of the bogie and its key components in a timely and accurate manner directly determines the operational performance and safety of the EMU \[[70](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0068)\]. Therefore, it has high quality and maintenance service requirements. In addition, the bogie also has the characteristics of a complex structure. For example, the bogie generally consists of a wheelset, axle box, [bogie frame](https://www.sciencedirect.com/topics/engineering/bogie-frame "Learn more about bogie frame from ScienceDirect's AI-generated Topic Pages"), mechanical drive unit, foundation brake rigging, primary suspension, secondary suspension, etc. Each component is made up of thousands of parts. Meanwhile, all the abovementioned tasks of the EMU need to be realised by the complex movement relations between different components of the bogie. Therefore, the EMU bogie is a typical complex product. [Fig. 4](https://www.sciencedirect.com/science/article/pii/S0736584522001673#fig0004) is a schematic diagram of the H38L (a virtual name used in this paper) high-speed EMU bogie manufactured by the partner company.

![Fig 4](https://ars.els-cdn.com/content/image/1-s2.0-S0736584522001673-gr4.jpg)

1.  [Download : Download high-res image (533KB)](https://ars.els-cdn.com/content/image/1-s2.0-S0736584522001673-gr4_lrg.jpg "Download high-res image (533KB)")
2.  [Download : Download full-size image](https://ars.els-cdn.com/content/image/1-s2.0-S0736584522001673-gr4.jpg "Download full-size image")

Fig. 4. The schematic diagram of the H38L high-speed EMU bogie.

In the past, since local railway companies operated the EMUs in different geographical areas, the operational status data of EMU and their critical components in diverse conditions could not be acquired accurately and completely. As a result, the efficiency of OM service schemes for EMU based on incomplete and inaccurate operational status data was lower, which has directly affected the safety and service life of EMU. Meanwhile, the knowledge that affects fault analysis and service decision-making is difficult to reuse due to the unreasonable organisation and management of historical OM knowledge. This has made active pushing of OM service cases extremely difficult to realise.

Therefore, it is important to determine how to enhance the accuracy and efficiency of OM service cases pushing and schemes decision-making for EMU to improve safety and reliability, which was the main challenge the partner company faced for a long period. Recently, with the rapid advancement of portrait technology and the wide application of PSS mode, the company sought a new way to achieve the potential of applying the lifecycle data and historical OM knowledge of EMUs for fault analysis and OM scheme decision-making and to transform its manufacturing mode from the product-driven pattern to the system-integration and the service-driven one. Therefore, they tested the novel approach according to [Section 3](https://www.sciencedirect.com/science/article/pii/S0736584522001673#sec0006). The proposed approach was tested and followed the combination of quantitative and qualitative methods based on case analysis and semi-structured interviews.

### 5.2. Establishing of bogie multi-level OM service case library

The failure reporting analysis and corrective action system (FRACAS) is the main tool developed by the partner company and its service providers to manage the OM data of EMUs within the service-driven manufacturing mode. In the FRACAS, the fault record data of the operation process and maintenance site for the geographically scattered EMU are included, which is analysed in this paper to excavate the fault information of bogie. The fault information of the EMU bogie is automatically triggered by the activation of an alarm function that setting in the FRACAS.

The FRACAS can record multiple fields related to EMU fault information (e.g., date of fault, EMU number, fault description, fault cause, fault effect, troubleshooting measure, responsible department, etc.). Based on this information, the working principle and operating environment of the EMU are further aligned to construct the fault system of the EMU bogie (as shown in [Table 3](https://www.sciencedirect.com/science/article/pii/S0736584522001673#tbl0003)).

Table 3. The fault system of EMU bogie.

| **Critical component system fault** | **Sub-system fault** | **Parts fault** |
| --- | --- | --- |
| The EMU bogie | Wheelset | Wheel tread fault |
| Wheel flange wearing |
| Axle fault |
| Brake disc fault |
| Axle box and locating device | Axle box damage |
| Axle box bearing failure |
| Axle end-cover fault |
| Axial positioning device fault |
| Bogie frame | Frame weld opening, frame crack |
| Frame deformation |
| Mechanical drive unit | Gearbox fault |
| Shaft coupling fault |
| Traction motor fault |
| Foundation brake unit | Brake pad fault |
| Balance slide bar failure |
| Bellow break |
| Brake disc crack |
| Brake cylinder fault |
| Primary suspension | Axle box spring failure |
| Rubber mat falling off |
| Vertical damper fault |
| Secondary suspension | Air spring fault |
| Levelling valve fault |
| Differential pressure valve fault |
| Anti-roll bar fault |
| Lateral damper fault |

Based on the EMU bogie fault system in [Table 3](https://www.sciencedirect.com/science/article/pii/S0736584522001673#tbl0003), the bogie's structure and failure mode are analysed by applying [FMEA](https://www.sciencedirect.com/topics/engineering/failure-mode-analysis "Learn more about FMEA from ScienceDirect's AI-generated Topic Pages") to construct the fault tree model of the EMU bogie (as shown in [Fig. 5](https://www.sciencedirect.com/science/article/pii/S0736584522001673#fig0005)). The symbol and definition of each event in the fault tree model are shown in [Table 4](https://www.sciencedirect.com/science/article/pii/S0736584522001673#tbl0004).

![Fig 5](https://ars.els-cdn.com/content/image/1-s2.0-S0736584522001673-gr5.jpg)

1.  [Download : Download high-res image (221KB)](https://ars.els-cdn.com/content/image/1-s2.0-S0736584522001673-gr5_lrg.jpg "Download high-res image (221KB)")
2.  [Download : Download full-size image](https://ars.els-cdn.com/content/image/1-s2.0-S0736584522001673-gr5.jpg "Download full-size image")

Fig. 5. The fault tree model of EMU bogie.

Table 4. The symbol and definition of each event in the fault tree model.

| Symbol | Event name | Symbol | Event name |
| --- | --- | --- | --- |
| T | Bogie fault | X11 | Gearbox fault |
| M1 | Wheelset fault | X12 | Shaft coupling fault |
| M2 | Axle box and locating device fault | X13 | Traction motor fault |
| M3 | Bogie frame fault | X14 | Brake pad wearing out |
| M4 | Mechanical drive unit fault | X15 | Brake pad clearance transfinite |
| M5 | Foundation brake unit fault | X16 | Split pin falling off |
| M6 | Primary suspension fault | X17 | Brake pad casting defects |
| M7 | Secondary suspension fault | X18 | Brake pad position abnormal |
| M8 | Axle box fault | X19 | Balance slide bar fracture |
| M9 | Brake calliper device fault | X20 | Balance slide block fall out |
| M10 | Brake pad fault | X21 | Bellow break |
| M11 | Balance slide bar failure | X22 | Brake disc fault |
| X1 | Wheel tread fault | X23 | Brake cylinder fault |
| X2 | Wheel flange wearing | X24 | Axle box spring failure |
| X3 | Axle fault | X25 | Rubber mat falling off |
| X4 | Brake disc fault | X26 | Vertical damper fault |
| X5 | Axle box damage | X27 | Air spring fault |
| X6 | Axle box bearing failure | X28 | Levelling valve fault |
| X7 | Axle end-cover fault | X29 | Differential pressure valve fault |
| X8 | Axial positioning device fault | X30 | Anti-roll bar fault |
| X9 | Frame weld opening, frame crack | X31 | Lateral damper fault |
| X10 | Frame deformation |  |  |

According to the fault tree model in [Fig. 5](https://www.sciencedirect.com/science/article/pii/S0736584522001673#fig0005), the multi-level OM service case library of the bogie is established by applying the method introduced in [Section 4.1.1](https://www.sciencedirect.com/science/article/pii/S0736584522001673#sec0009). Moreover, the text processing method in [Section 4.1.2](https://www.sciencedirect.com/science/article/pii/S0736584522001673#sec0010) is adopted to extract the _FSK_ and the possible fault causes of each sub-case library. Finally, a multi-level OM service case library is obtained. Detailed information is given in [Table 1](https://www.sciencedirect.com/science/article/pii/S0736584522001673#tbl0010) of the Appendix. Based on the bogie's multi-level OM service case library, the method in [Section 4.1.2](https://www.sciencedirect.com/science/article/pii/S0736584522001673#sec0010) is used to define and express the OM service case. As a result, the bogie OM service case structure can be represented as a table (as shown in [Table 5](https://www.sciencedirect.com/science/article/pii/S0736584522001673#tbl0005)).

Table 5. The structure of the bogie OM service case.

<table><tbody><tr><td id="en0210" rowspan="5">Basic information on the OM case</td><td id="en0211" colspan="2">Case number</td></tr><tr><td id="en0213" colspan="2">Sub-case library</td></tr><tr><td id="en0215" colspan="2">Fault occurrence time</td></tr><tr><td id="en0217" colspan="2">Fault occurrence location</td></tr><tr><td id="en0219" colspan="2">Fault symptom description text</td></tr><tr><td id="en0220" rowspan="5">Feature set of OM case</td><td id="en0221" colspan="2">Fault symptom feature vector</td></tr><tr><td id="en0223" rowspan="4">Attribute and behaviour feature vector</td><td id="en0224">Train type</td></tr><tr><td id="en0227">Train marshalling number</td></tr><tr><td id="en0230">Train number</td></tr><tr><td id="en0233">Carriage number…</td></tr><tr><td id="en0234" rowspan="3">Fault conclusion information</td><td id="en0235" colspan="2">Fault handling time</td></tr><tr><td id="en0237" colspan="2">Fault handling personnel</td></tr><tr><td id="en0239" colspan="2">Suggested solution</td></tr></tbody></table>

The suggested solution in [Table 5](https://www.sciencedirect.com/science/article/pii/S0736584522001673#tbl0005) is provided in text form through the OM service structured document platform that was developed in the FRACAS, where the information on bogie structure trees, maintenance types, and maintenance operation structured files can be presented. In particular, the detailed operation instructions of the suggested solution text can be provided in the maintenance operation structured files. For example, the information in maintenance operation structured files mainly includes maintenance task name, operation number, personnel and labour-hour, task description, spare parts, operation instruction text, etc. According to this information, maintenance instructions can be provided for field personnel.

[Table 6](https://www.sciencedirect.com/science/article/pii/S0736584522001673#tbl0006) shows an example of the original record of an EMU bogie OM service case. Based on the original case record, fault categories can be analysed and judged by the method of fault symptom keywords extracted introduced in [Section 4.1.2](https://www.sciencedirect.com/science/article/pii/S0736584522001673#sec0010). Finally, the OM service cases are expressed and managed by the structure represented in [Table 5](https://www.sciencedirect.com/science/article/pii/S0736584522001673#tbl0005), as shown in [Table 7](https://www.sciencedirect.com/science/article/pii/S0736584522001673#tbl0007). This mode can be further used to provide knowledge support for the active pushing of personalised OM service schemes for bogie and to promote the achievement of knowledge reuse.

Table 6. An example of the original record of an EMU bogie OM service case.

<table><tbody><tr><th id="en0240" scope="row">Original case record</th><td id="en0241">At 22:35:00, August 4, 2018, the 03rd carriage of train G323 from HF to WZ belongs to 3875 marshalling and H38L type train at kilometre post K129+39, the fault of wheel tread scrape, wheel wear and local dent, wheel flange transfinite wearing occurred.</td></tr></tbody></table>

Table 7. An example of the expression of the EMU bogie OM service case.

| **Case expression** | **Case details** |
| --- | --- |
| **Case basic information** | Case number | Case library | Sub-case library | Occurrence time | Occurrence location |
| C01–024,581 | Bogie OM service case library | Wheel tread fault sub-case library | August 4, 2018, 22:35:00 | At kilometre post K129+39 |
| **Case feature set** | FSFV | {wheel tread scrape: 0.4, wheel local dent:0.35, wheel flange wearing: 0.25} |
| ABFV | FC | FP | TT | TN | TMN | CN | Mi /km | EAT / °C | TC | RF/SF |
| Wheelset fault | Wheel tread | H38L | G323 | 3875 | 03 | 191,421 | 30 | plain | yes |
| **Fault conclusion information** | Fault handling time | Fault handling personnel | Suggested solution |
| August 5, 2018, 08:00:00 | Zhang San | The fault is caused by the electronic antiskid device's failure due to the vehicle's excessive braking force. The wheel lathe needs to be replaced and trimmed to the circular. Then repair and maintain the electronic antiskid device to work normally to avoid the wheel scrape fault. |

FSFV-fault symptom feature vector; ABFV-attribute and behaviour feature vector; FC-fault category; FP-faulty position; TT-train type; TN-train number; TMN-train marshalling number; CN-carriage number; Mi-Mileage; EAT-environmental average temperature; TC-terrain condition; RF-rainfall; SF-snowfall.

### 5.3. Active pushing of OM service cases for bogie wheelset

Taking the wheelset of the bogie for the H38L high-speed EMU as an example, the feasibility and effectiveness of the active pushing method of OM service cases are verified in this section. Given the equipment portrait model of the bogie is _M<sub>u</sub>_\= {(_F_<sub>1</sub>, _E_<sub>1</sub>): _ω_<sub>1</sub>, (_F_<sub>2</sub>, _E_<sub>2</sub>): _ω_<sub>2</sub>, …, (_F<sub>n</sub>, E<sub>n</sub>_): _ω<sub>n</sub>_}, and assumed that (_F_<sub>1</sub>, _E_<sub>1</sub>) represents the fault category feature vector of the wheelset. When a certain fault of the wheelset occurred, the FRACAS will extract the portrait model of the wheelset and the sub-case library that corresponds to this fault type. Then, the similarity calculation instruction will be further triggered in the FRACAS to calculate the similarity between the fault category feature vector and OM service case in the sub-case library to get the best similarity cases. The best similar case corresponds to an OM service solution of the fault type for the bogie wheelset. Therefore, the suggested solution text should be extracted from the previously established OM service case library based on the occurred fault type.

In this application scenario, the fault feature vector of the wheelset in the target case is expressed as _F_<sub>1</sub>\= {wheel tread scrape: 0.5, wheel indentation depth transfinite: 0.3, wheel wear: 0.2}. Furthermore, the attribute and behaviour feature vector are expressed as _E_<sub>1</sub>\= {TT: H38L, TN: G372, TMN: 3736, CN: 02, mileage: 160,782, EAT: 27, TC: hills, RF or SF: yes, FC: wheelset fault, FP: wheel tread}. [Table 8](https://www.sciencedirect.com/science/article/pii/S0736584522001673#tbl0008) shows the cases related to the fault case library of the EMU bogie wheelset. Meanwhile, [Table 2](https://www.sciencedirect.com/science/article/pii/S0736584522001673#tbl0011) of the Appendix shows the value types of the feature items of the EMU bogie fault case library.

Table 8. Cases related to the fault case library of bogie wheelset.

| **Feature items** | **Case 1** | **Case 2** | **Case 3** | **Case 4** |
| --- | --- | --- | --- | --- |
| **F** | FSFV | {wheel rim crack: 0.7, wear: 0.3} | {wear: 0.3, sparking: 0.35, brake shoe deformation: 0.25} | {wheel tread scrape: 0.4, abnormal noise: 0.35, wear: 0.25} | {wheel tread scrape: 0.15, wheel tread peeling: 0.85} |
| **E** | TT | H380L | H40F | H38L | H30C |
| TN | G372 | G506 | G434 | G650 |
| TMN | 3651 | 3013 | 3782 | 3023 |
| CN | 02 | 06 | 03 | 07 |
| Mi/km | 113,500 | 57,805 | 191,421 | 132,560 |
| EAT/ °C | 12 | 20 | 30 | 26 |
| TC | Hills | Plain | Plain | Hills |
| RF or SF | No | No | Yes | Yes |
| FC | Wheelset fault | Wheelset fault | Wheelset fault | Wheelset fault |
| FP | Wheel rim | Wheel tread | Wheel tread | Wheel tread |
| **Suggested solution** | This fault is caused by local material fatigue. If the wheel rim crack is not more than 80 mm, the train can directly run to the destination after inspecting and affirming by manage department. If the crack exceeds 80 mm, the carriage should be disengaged from the train immediately to ensure the safety of the whole EMU. | This fault is caused by the poor material of the brake pad. For example, the adhesive, filler or auxiliary material does not conform to regulation. The rail transit equipment company should strictly screen their suppliers and control the quality of parts. | The wheel tread scrape and wear are caused by the failure of the electronic antiskid device due to the excessive braking force of the train. The wheel lathe needs to be replaced and trimmed to the circular. Then repair the electronic antiskid device to work properly and avoid the wheel scrape. The fixed frequency abnormal noise is caused by local depression of the wheel tread due to the too-soft material of the wheelset and foreign object impact. This problem needs to be fed back to the wheelset supplier to adjust the material. | This fault is related to the material's low yield and tensile strength. The wheelset needs to be replaced. Then the wheel lathe needs to be trimmed to improve the smelting quality and to reduce the metal wear caused by tread peeling. |

By using the similarity calculation method in [Section 4.2.2](https://www.sciencedirect.com/science/article/pii/S0736584522001673#sec0017), four cases in [Table 8](https://www.sciencedirect.com/science/article/pii/S0736584522001673#tbl0008) are matched. Meanwhile, the weight value of each feature item is {0.5, 0.05, 0.03, 0.01, 0.01, 0.1, 0.04, 0.03, 0.03, 0.1, 0.1}. That is, the weight ρF of _F_<sub>1</sub> is 0.5, and the weight ρE of _E_<sub>1</sub> is 0.5. Meanwhile, set the mileage range to \[0, 1,800,000\], the lower limit of operation environment temperature is −30 °C, and the upper limit is 40 °C. Finally, the case matching results ranked from high to low by global similarity is shown in [Table 9](https://www.sciencedirect.com/science/article/pii/S0736584522001673#tbl0009).

Table 9. The OM case matching results.

| The case number | The global similarity of case |
| --- | --- |
| Case 3 | 0.8165 |
| Case 4 | 0.7158 |
| Case 1 | 0.6971 |
| Case 2 | 0.6372 |

As seen in [Table 9](https://www.sciencedirect.com/science/article/pii/S0736584522001673#tbl0009), Case 3 has the greatest global similarity. Therefore, Case 3 can be suggested and considered the best similar case for the target product, and its solution has the most reference value. And the solution of Case 3 can be pushed to field personnel, and then the OM service for the wheelset can be carried out following the generated maintenance operation structured files. However, the fault contained in the solution of case 3 is different from the target product. For example, there is no abnormal noise problem for the target product. Thereby, the personalised OM service solution for the target product does not contain measures for [noise cancellation](https://www.sciencedirect.com/topics/engineering/noise-cancellation "Learn more about noise cancellation from ScienceDirect's AI-generated Topic Pages"). That is, the suggested solution should be adjusted according to the actual OM situation through artificial intervention. As a result, the adjusted OM service scheme is replacing the wheel lathe and trimming it to the circular, repairing the electronic antiskid device to work normally, and avoiding the wheel scrape.

The above-mentioned processes illustrated that the way for the personnel to confirm or dismiss the usefulness of the suggested solution is involved and considered in the application scenario study. According to the method and process described in [Section 4.2.2.2](https://www.sciencedirect.com/science/article/pii/S0736584522001673#sec0019), experienced maintenance personnel will intervene in the judgement and [verification processes](https://www.sciencedirect.com/topics/computer-science/verification-process "Learn more about verification processes from ScienceDirect's AI-generated Topic Pages") of the usefulness of the OM service case. As a result, the edited and adjusted OM service case will be dynamically updated to the sub-case library. Therefore, if the suggested solution is not the right one, the FRACAS will generate and recommend the new solution through the OM service structured document platform based on personnel confirming and editing the solution content. The interviews confirmed that the partner company adopts the human-based manner to improve operational decision-making in maintenance service delivery processes based on the actual OM service situation. Feedback to improve and adjust the system-based solution proposal for the introduction of new ones according to the actual maintenance task is necessary.

### 5.4. Analysis and discussions

The study of the application scenario provides strong evidence that the proposed approach is valid and feasible, which indicates that it has the potential to be applied in the industry field for product OM services. Compared with other research \[[25](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0024),[26](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0025),[30](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0029)\], the major difference is the novel combination of PSS and portrait technology with the possibility of acquiring more comprehensive lifecycle data and the advantage of describing personalised user requirements and providing refined product services.

The proposed novel POMA-CP based on equipment portrait within the PSS mode makes it possible to access the product data of different [lifecycle stages](https://www.sciencedirect.com/topics/computer-science/lifecycle-stage "Learn more about lifecycle stages from ScienceDirect's AI-generated Topic Pages") and trace the business data of different stakeholders. For example, the product data at the beginning of life (design and manufacturing stages), middle of life (operation, service and maintenance stages) and end of life (remanufacturing, recycling, reuse), and the business data scattered in OEMs, component suppliers, maintenance service providers, product operators, etc. These data create effective means to enable inter-connect user needs, personalised OM and sustainable competitive advantage. For example, the abovementioned data can be analysed to construct the fine-grained label library and accurate portrait model to manage better and maintain complex products while improving different stakeholders’ business processes and decisions. As a result, the complex products can be run in a better state, thereby enhancing customer satisfaction and promoting user buyback and [user experience](https://www.sciencedirect.com/topics/computer-science/user-experience "Learn more about user experience from ScienceDirect's AI-generated Topic Pages"). This would be a potential competitive advantage for OEMs and service providers.

The proposed approach, along with the multi-source data of different lifecycle stages and stakeholders, allows the service providers to find more hidden fault features and match more precise OM service schemes in a shorter time. This will contribute service providers to reducing maintenance costs and resource consumption, OEMs optimising product design and [manufacturing process](https://www.sciencedirect.com/topics/engineering/production-process "Learn more about manufacturing process from ScienceDirect's AI-generated Topic Pages"), and product users reducing safety risks caused by a sudden failure. Therefore, the PSS mode, rich data, and personalised OM approach are three significant pillars for exploring the potential of the equipment portrait for complex products. These three elements provide insights to service providers and OEMs to develop and implement [sustainable business models](https://www.sciencedirect.com/topics/engineering/sustainable-business-model "Learn more about sustainable business models from ScienceDirect's AI-generated Topic Pages") in line with corporate social responsibility.

According to the semi-structured interviews with the managers of the partner company, in the past, even though mainly operational data of the critical components for EMU bogie were collected and analysed, the deviation between the maintenance decision and task execution is often produced. This is mainly caused by the dynamic changing of bogie operational status, fuzzy relationship, and inconsistent data transmission amongst different lifecycle stages and stakeholders. By applying the novel OM approach with multi-source lifecycle data and dynamic attribute and behaviour label library, more fault features and knowledge were found to conduct targeted analysis on bogie's operational status and performance. Therefore, personalised, refined and active OM services can be carried out to ensure the safety and reliability of EMU. This can move the service providers from planned [corrective maintenance](https://www.sciencedirect.com/topics/engineering/corrective-maintenance "Learn more about corrective maintenance from ScienceDirect's AI-generated Topic Pages") to proactive and condition-based OM service planning [\[6\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bib0006), reducing the service providers’ [OM costs](https://www.sciencedirect.com/topics/engineering/operation-and-maintenance-cost "Learn more about OM costs from ScienceDirect's AI-generated Topic Pages") and ensuring the customers’ safety. The managers of the partner company also emphasised that the schemes of the bogie OM service case for frequent faults can provide new ideas and insights for research and development of the next generation of the EMU bogie. Moreover, other potential benefits of the novel approach, such as the personalised and refined OM service for [product families](https://www.sciencedirect.com/topics/computer-science/product-family "Learn more about product families from ScienceDirect's AI-generated Topic Pages"), can be realised because of the rich portrait models.

## 6\. Conclusions

Many manufacturing enterprises of complex products have transformed their businesses towards PSS modes and have integrated user portraits with relevant OM services, which helps accelerate the transition of service mode to refined, differentiated, [personalised services](https://www.sciencedirect.com/topics/computer-science/personalized-service "Learn more about personalised services from ScienceDirect's AI-generated Topic Pages"). As a result, the concept of equipment portrait has received increasing attention in complex products’ OM service studies. Meanwhile, the opportunities for OEMs to accurately describe personalised user demands by comprehensive collecting the data of PSS delivery processes for complex products are continuously increasing. However, with the [permeation](https://www.sciencedirect.com/topics/engineering/permeation "Learn more about permeation from ScienceDirect's AI-generated Topic Pages") and application of portrait technology in industrial and complex products’ OM service fields, OEMs are facing many challenges. For example, how should a prescriptive procedure for equipment portrait-based OM service delivery and execution processes of industrial complex products be structured, and how to effectively organise and reuse the knowledge extracted from the PSS delivery process data to support equipment portrait-based OM service decision-making?

In order to address the challenges, in this paper, a novel framework and solution of POMA-CP based on equipment portrait within the PSS mode was proposed. First, the proposed POMA-CP framework contributes to the research related to the investigation of a prescriptive and structured process for the implementation of the equipment portrait-based OM service decision-making. The framework answers RQ1 by guiding the industrial practitioners in exploiting data generated during the whole PSS delivery lifecycle to improve and optimise the OM service delivery and execution processes (e.g., personalised and refined OM service), then addressing one of the knowledge gaps identified from existing literature (as stated in paragraph 2 of [Section 2.3](https://www.sciencedirect.com/science/article/pii/S0736584522001673#sec0005)). Second, another knowledge gap identified from the literature review existed in the lack of approaches proposed for exploiting the integrated application of equipment portrait and PSS (as stated in paragraph 3 of [Section 2.3](https://www.sciencedirect.com/science/article/pii/S0736584522001673#sec0005)). The solution of POMA-CP contributes to addressing this gap and answers RQ2 by introducing a set of methods in each component aimed at organizing and managing the knowledge excavated from PSS delivery processes data to establish a precise portrait model, and support the industrial practitioners in personalised and refined OM service decision-making.

The limitations of this paper are summarised as follows. First, in the application scenario, only one case record (see [Table 6](https://www.sciencedirect.com/science/article/pii/S0736584522001673#tbl0006)) and one bogie (see [Table 8](https://www.sciencedirect.com/science/article/pii/S0736584522001673#tbl0008)) for a specific EMU was used to test the effectiveness of the active pushing mechanism of the OM service solution. To strengthen the applicability and robustness of the proposed approach, different components for this application scenario and different types of EMU bogies as well as different operational conditions (or case records) should be considered and tested comprehensively. Second, when organising and expressing the OM service case of EMU bogie, only ten mainly features (see [Table 7](https://www.sciencedirect.com/science/article/pii/S0736584522001673#tbl0007)) that reflect bogie fault features were selected to construct the attribute and behaviour feature vector. In order to improve the efficiency and accuracy of case matching, more features should be selected and considered to enrich the case library's information and provide more fine-grained information or knowledge for personalised and refined OM services. Third, the system-based and human-based manner are combined together in the application scenario study to generate the best similarity cases and to suggest the final OM service solutions. However, the validation test comparing human-based and system-based solution proposals and the time required to evaluate the proposed solution were not performed in this work.

Future research will focus on the following three aspects. First, the labels used to establish the equipment portrait model will dynamically change along with the different operational conditions of complex products. Therefore, a periodic updating mechanism (as stated in [Section 3](https://www.sciencedirect.com/science/article/pii/S0736584522001673#sec0006)) of the equipment portrait model should be investigated to implement more refined OM services. Second, with the ever-increasing accumulation of equipment operational data and OM service abnormal event log text, more advanced algorithms should be considered to improve the accuracy of fault symptom keywords extracting and fault cause identification (as stated in [Section 4.2](https://www.sciencedirect.com/science/article/pii/S0736584522001673#sec0013)). Third, more abundant similarity calculation methods for different types of label values (as stated in [Section 4.2.2](https://www.sciencedirect.com/science/article/pii/S0736584522001673#sec0017)) should be developed and involved to maximise historical OM service knowledge's [reusability](https://www.sciencedirect.com/topics/engineering/reusability "Learn more about reusability from ScienceDirect's AI-generated Topic Pages") and enhance the precision of OM service case-pushing and scheme decision-making.

## CRediT authorship contribution statement

**Shan Ren:** Writing – original draft, Methodology, Software. **Lichun Shi:** Methodology, Software. **Yang Liu:** Supervision, Validation, Writing – review & editing. **Weihua Cai:** [Data curation](https://www.sciencedirect.com/topics/computer-science/data-curation "Learn more about Data curation from ScienceDirect's AI-generated Topic Pages"). **Yingfeng Zhang:** Supervision, Validation.

## Declaration of Competing Interest

The authors declare that they have no known competing financial interests or personal relationships that could have appeared to influence the work reported in this paper.

## Acknowledgements

The authors would like to acknowledge the financial support of the National Natural Science Foundation of China (Grant No. [52005408](https://www.sciencedirect.com/science/article/pii/S0736584522001673#gs0001) and [U2001201](https://www.sciencedirect.com/science/article/pii/S0736584522001673#gs0001)) and the Key Research and Development Program of Shaanxi ([2021GY-069](https://www.sciencedirect.com/science/article/pii/S0736584522001673#gs0002)).

## Appendix

Table A.1. The bogie multi-level OM service case library and text processing results (examples).

| Critical component system OM service case library | Sub-case library of OM service | Fault symptom keywords | Fault causes |
| --- | --- | --- | --- |
| Faulty position | Sub-case library |
| --- | --- |
| Bogie OM service case library | Wheelset | Wheel tread fault | Tread scrape, peeling, wear, wheel indentation depth transfinite, thermal crack, wheel diameter deformation transfinite, … | ① fierce friction with the rail  
② tread deformation  
③ operating environment impact  
④ bad material  
⑤ wheelset wear out  
… |
| Wheel flange wearing | Wheel flange crack, flange thickness decreased, burr at the flange, vertical wear, … | ① bad material  
② excessive wear  
③ tread dents  
… |
| Brake disc fault | Bolt slipping off, rubber gasket deformation, … | ① vibration fatigue failure  
② stuck by foreign object  
… |
| Axle fault | Axle body scratch, axle end dust cover crack, scrap iron at internal thread, axle body bulge, axle temperature increase, seal ring fracture, hollow axle ash deposition, … | ① foreign object scratch  
② manufacturing defect  
③ improper processing technic  
④ operating environment impact  
⑤ excessive wear  
⑥ vibration fatigue failure  
… |
| Axle box and locating device | Axle box damage | Load bearing saddle bolt looseness, oil leak, … | ① vibration fatigue failure  
② wear out, fracture  
… |
| Axle box bearing failure | Bearing cup failure, bearing cone failure, bearing roller failure, bearing retainer failure, lubricating system failure, crack and defect, scrape and scratch, surface plastic deformation, corrosion, … | ① bad material  
② improper heat treatment  
③ poor assembly  
④ poor lubrication  
⑤ improper seal  
⑥ impurity in bearings  
⑦ bearing overload  
… |
| Axle end-cover fault | Damage, fracture, crack, scratch, bolt fracture, rubber-cover looseness, … | ① improper use and maintenance  
② contact fatigue  
③ mechanical movement unreasonable  
④ improper manual operation  
… |
| Axial positioning device fault | Positioning node bolt looseness, positioning seat crack, positioning bolt gasket crack, locating ring fall off, locating pin missing, locating sleeve axial endplay, … | ① vibration fatigue failure  
② mechanical movement unreasonable  
③ unreasonable design  
… |
| Bogie frame | Frame fault | Air spring mounting hole scratch, lateral damper mounting seat tilting, brake beam hanger seat scrape, traction bar mounting seat uneven, anti-snake damper mounting base crack, lateral control lever positioning pin bolt looseness, lateral control lever safety bolt missing, … | ① contact fatigue  
② contact stress frequency  
③ improper processing technic  
④ manufacturing defect  
⑤ excessive wear  
⑥ vibration fatigue failure  
… |
| Mechanical drive unit | Gearbox fault | Gear crack, exudation oil, shaft temperature increase, scrap iron in gearbox, … | ① vibration fatigue failure  
② improper processing technic  
③ unreasonable mechanical movement  
④ manufacturing defect  
… |
| Shaft coupling fault | Shaft coupling crack, shaft coupling damage, shaft coupling paint drop, … | ① excessive wear  
② contact fatigue  
③ manufacturing defect  
… |
| Traction motor fault | Motor oil leakage, motor temperature high, excessive motor vibration, abnormal motor noise, sealant fall off, snap ring fracture, cooling fan wiring looseness, cooling fan contactor fault, … | ① manufacturing defect  
② ash deposition on the surface of blower impeller  
③ vibration fatigue failure  
④ operating environment impact  
⑤ improper processing technic  
… |
| Foundation brake unit fault | Brake pad fault | Brake pad wearing out, brake pad clearance transfinite, split pin falling off, brake pad casting defects, brake pad position abnormal, … | ① service wear  
② unreasonable mechanical movement  
③ vibration fatigue failure  
④ improper processing technic  
… |
| Brake calliper device fault | Balance slide bar fracture, balance slide block fall out, bellow break, brake calliper arm jamming, guiding device crack, brake calliper casting defect, … | ① stuck by foreign object  
② vibration fatigue failure  
③ impact and collision  
④ unreasonable mechanical movement  
⑤ improper processing technic  
… |
| Brake disc fault | Brake disc crack transfinite, … | ① improper use and maintenance  
… |
| Brake cylinder fault | Brake cylinder wearing out, … | ① unreasonable mechanical movement  
… |
| Primary suspension | Axle box spring failure | Axle box spring fracture, deformation, looseness, wear out, … | ① spring crack or improper assembly  
… |
| Rubber mat falling off | Rubber mat falling off, … | ① vibration fatigue failure  
… |
| Vertical damper fault | Vertical damper oil leakage, vertical damper damage, … | ① manufacturing or assembly defect  
② stuck by foreign object  
… |
| Secondary suspension | Air spring fault | Air spring crack, air spring burst, air spring air leakage, air spring jamming, adjusting lever falling off, adjusting lever ball joint jamming, … | ① inlet pressure low  
② improper processing technic  
③ surface foreign object wear out  
④ vibration fatigue failure  
⑤ unreasonable mechanical movement  
… |
| Levelling valve fault | Levelling valve oil leakage, levelling valve rotation abnormal, adjusting lever deformation, adjusting lever crack, … | ① excessive wear or high stress  
② unreasonable mechanical movement  
③ contact fatigue  
… |
| Differential pressure valve fault | Differential pressure valve nut looseness, … | ① vibration fatigue failure  
… |
| Anti-roll bar fault | Rubber ring deformation, anti-roll bar looseness, anti-roll bar abnormal noise, anti-roll bar crack, anti-roll bar jamming, axial bushing endplay, … | ① vibration fatigue failure  
② contact stress high  
③ excessive wear  
④ unreasonable mechanical movement  
… |
| Lateral damper fault | Lateral damper oil leakage, lateral damper rubber deformation, … | ① manufacturing or assembly defect  
② excessive wear  
… |

Table A.2. The value types of the feature items for EMU bogie fault case library.

| Feature items | Value types |
| --- | --- |
| FSFV | Text |
| TT | Symbol |
| TN | Symbol |
| TMN | Symbol |
| CN | Symbol |
| Mi | Interval number |
| EAT | Numerical value |
| TC | Symbol |
| RF or SF | Symbol |
| FC | Symbol |
| FP | Symbol |

## Data availability

-   The authors do not have permission to share data.
    

## References

1.  [\[1\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0001)
    
    Product portfolio architectural complexity and operational performance: incorporating the roles of learning and fixed assets
    
2.  [\[2\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0002)
    
    R.R. Inman, D.E. Blumenfeld
    
    Product complexity and supply chain design
    
3.  [\[3\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0003)
    
    A. Trattner, L. Hvam, C. Forza, Z.N.L. Herbert-Hansen
    
    Product complexity and operational performance: a systematic literature review
    
4.  [\[4\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0004)
    
    H. Zhu, J. Gao, D. Li, D. Tang
    
    A Web-based product service system for aerospace maintenance, repair and overhaul services
    
5.  [\[5\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0005)
    
    Y. Yi, Y. Yan, X. Liu, Z. Ni, J. Feng, J. Liu
    
    Digital twin-based smart assembly process design and application framework for complex products and its case study
    
6.  [\[6\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0006)
    
    N. Wang, S. Ren, Y. Liu, M. Yang, J. Wang, D. Huisingh
    
    An active preventive maintenance approach of complex equipment based on a novel product-service system operation mode
    
7.  [\[7\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0007)
    
    M. Behzad, H. Kim, M. Behzad, H. Asghari Behambari
    
    Improving sustainability performance of heating facilities in a central boiler room by condition-based maintenance
    
8.  [\[8\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0008)
    
    S. Ren, Y. Zhang, T. Sakao, Y. Liu, R. Cai
    
    An Advanced Operation Mode with Product-Service System Using Lifecycle Big Data and Deep Learning
    
9.  [\[9\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0009)
    
    G. Yadav, S. Luthra, S.K. Jakhar, S.K. Mangla, D.P. Rai
    
    A framework to overcome sustainable supply chain challenges through solution measures of industry 4.0 and circular economy: an automotive case
    
10.  [\[10\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0010)
    
    S. Ma, W. Ding, Y. Liu, S. Ren, H. Yang
    
    Digital twin and big data-driven sustainable smart manufacturing based on information management systems for energy-intensive industries
    
11.  [\[11\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib69)
    
    E. Flores-García, Y. Jeong, S. Liu, M. Wiktorsson, L. Wang
    
    Enabling industrial internet of things-based digital servitization in smart production logistics
    
12.  [\[12\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0011)
    
    P. Zheng, Z. Wang, C.H. Chen, L.Pheng Khoo
    
    A survey of smart product-service systems: key aspects, challenges and future perspectives
    
13.  [\[13\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0012)
    
    P. Zheng, X. Xu, C.H. Chen
    
    A data-driven cyber-physical approach for personalised smart, connected product co-development in a cloud-based environment
    
14.  [\[14\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0013)
    
    K. Exner, C. Schnürmacher, S. Adolphy, R. Stark
    
    Proactive maintenance as success factor for use-oriented product-service systems
    
15.  [\[15\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0014)
    
    D. Mourtzis, J. Angelopoulos, N. Boli
    
    Maintenance assistance application of Engineering to Order manufacturing equipment: a product service system (PSS) approach
    
16.  [\[16\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0015)
    
    S. Wan, D. Li, J. Gao, J. Li
    
    A knowledge based machine tool maintenance planning system using case-based reasoning techniques
    
17.  [\[17\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0016)
    
    F. Chang, G. Zhou, W. Cheng, C. Zhang, C. Tian
    
    A service-oriented multi-player maintenance grouping strategy for complex multi-component system based on game theory
    
18.  [\[18\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0017)
    
    J.S. Liang
    
    A process-based automotive troubleshooting service and knowledge management system in collaborative environment
    
19.  [\[19\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0018)
    
    W. Qin, Z. Zhuang, Y. Liu, J. Xu
    
    Sustainable service oriented equipment maintenance management of steel enterprises using a two-stage optimization approach
    
20.  [\[20\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0019)
    
    W. Luo, T. Hu, Y. Ye, C. Zhang, Y. Wei
    
    A hybrid predictive maintenance approach for CNC machine tool driven by Digital Twin
    
21.  [\[21\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0020)
    
    T. Xia, K. Zhang, B. Sun, X. Fang, L. Xi
    
    Integrated Remanufacturing and Opportunistic Maintenance Decision-Making for Leased Batch Production Lines
    
22.  [\[22\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0021)
    
    F. Anvari, D. Richards, M. Hitchens, M.A. Babar, H.M.T. Tran, P. Busch
    
    An empirical investigation of the influence of persona with personality traits on conceptual design
    
23.  [\[23\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0022)
    
    J. Salminen, K. Guan, S.G. Jung, B.J. Jansen
    
    A Survey of 15 Years of Data-Driven Persona Development
    
24.  [\[24\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0023)
    
    G. Antoniol, M. Di Penta
    
    A distributed architecture for dynamic analyses on user-profile data
    
25.  [\[25\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0024)
    
    Y. Huang, X. Wang, M. Gardoni, A. Coulibaly
    
    Task-Oriented Adaptive Maintenance Support System
    
26.  [\[26\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0025)
    
    S.L. Rautanen, P. White
    
    Portrait of a successful small-town water service provider in Nepal's changing landscape
    
27.  [\[27\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0026)
    
    J. Li, F. Tao, Y. Cheng, L. Zhao
    
    Big Data in product lifecycle management
    
28.  [\[28\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0027)
    
    P.A. Legg, O. Buckley, M. Goldsmith, S. Creese
    
    Automated Insider Threat Detection System Using User and Role-Based Profile Assessment
    
29.  [\[29\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0028)
    
    X.-.R. Liu, F.-.J. Zhang, Q.-.Y. Sun, P. Jin
    
    Location and Capacity Selection Method for Electric Thermal Storage Heating Equipment Connected to Distribution Network Considering Load Characteristics and Power Quality Management
    
30.  [\[30\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0029)
    
    C.H. Chung, S. Jangra, Q. Lai, X. Lin
    
    Optimization of Electric Vehicle Charging for Battery Maintenance and Degradation Management
    
31.  [\[31\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0030)
    
    D. Mourtzis, A. Vlachou, V. Zogopoulos
    
    Cloud-Based Augmented Reality Remote Maintenance Through Shop-Floor Monitoring: a Product-Service System Approach
    
32.  [\[32\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0031)
    
    V. Grover, R.H.L. Chiang, T.P. Liang, D. Zhang
    
    Creating Strategic Business Value from Big Data Analytics: a Research Framework
    
33.  [\[33\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0032)
    
    L.M. Ramirez Restrepo, S. Hennequin, A. Aguezzoul
    
    Optimization of integrated preventive maintenance based on infinitesimal perturbation analysis
    
34.  [\[34\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0033)
    
    K. Matyas, T. Nemeth, K. Kovacs, R. Glawar
    
    A procedural approach for realizing prescriptive maintenance planning in manufacturing industries
    
35.  [\[35\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0034)
    
    J. Cavalcante, L. Gzara
    
    Product-Service Systems lifecycle models: literature review and new proposition
    
36.  [\[36\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0035)
    
    A. Cooper
    
    Software-Ergonomie ’99. Berichte des German Chapter of the ACM
    
    The Inmates are Running the AsylumU. Arend, E. Eberleh, K. Pitschke (Eds.), 53, Vieweg+Teubner Verlag, Wiesbaden (1999), p. 17
    
    [https://doi.org/10.1007/978-3-322-99786-9\_1](https://doi.org/10.1007/978-3-322-99786-9_1)
    
37.  [\[37\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0036)
    
    S. Gauch, M. Speretta, A. Chandramouli, A. Micarelli
    
    User profiles for personalized information access
    
    Lect. Notes Comput. Sci. (Including Subser. Lect. Notes Artif. Intell. Lect. Notes Bioinformatics), Springer Berlin Heidelberg, Berlin, Heidelberg (2007), pp. 54-89,
    
    [10.1007/978-3-540-72079-9\_2](https://doi.org/10.1007/978-3-540-72079-9_2)
    
38.  [\[38\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0037)
    
    C. Teixeira, J. Sousa Pinto, J. Arnaldo Martins
    
    User profiles in organizational environments
    
39.  [\[39\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0038)
    
    S. Schiaffino, A. Amandi
    
    Intelligent user profiling, Lect. Notes Comput. Sci. (Including Subser. Lect. Notes Artif. Intell. Lect. Notes Bioinformatics)
    
40.  [\[40\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0039)
    
    V. Eyharabide, A. Amandi
    
    Ontology-based user profile learning
    
41.  [\[41\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0040)
    
    O. Hasan, B. Habegger, L. Brunie, N. Bennani, E. Damiani
    
    A discussion of privacy challenges in user profiling with big data techniques: the EEXCESS use case
    
42.  [\[42\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0041)
    
    R. Rossi, M. Gastaldi, F. Orsini
    
    How to drive passenger airport experience: a decision support system based on user profile
    
43.  [\[43\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0042)
    
    A.K. Sahu, P. Dwivedi
    
    User profile as a bridge in cross-domain recommender systems for sparsity reduction
    
44.  [\[44\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0043)
    
    R.M. Bertani, R.A.C. Bianchi, A.H.R. Costa
    
    Combining novelty and popularity on personalised recommendations via user profile learning
    
45.  [\[45\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0044)
    
    Z. Cheng, X. Zhang
    
    A novel intelligent construction method of individual portraits for WeChat users for future academic networks
    
46.  [\[46\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0045)
    
    S. Si, J. Zhao, Z. Cai, H. Dui
    
    Recent advances in system reliability optimization driven by importance measures
    
47.  [\[47\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0046)
    
    R. Palmarini, J.A. Erkoyuncu, R. Roy, H. Torabmostaedi
    
    A systematic review of augmented reality applications in maintenance
    
48.  [\[48\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0047)
    
    Y. Zhang, S. Ren, Y. Liu, S. Si
    
    A big data analytics architecture for cleaner manufacturing and maintenance processes of complex products
    
49.  [\[49\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0048)
    
    B. Marchi, S. Zanoni, L. Mazzoldi, R. Reboldi
    
    Product-service System for Sustainable EAF Transformers: real Operation Conditions and Maintenance Impacts on the Life-cycle Cost
    
50.  [\[50\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0049)
    
    F. Chang, G. Zhou, C. Zhang, Z. Xiao, C. Wang
    
    A service-oriented dynamic multi-level maintenance grouping strategy based on prediction information of multi-component systems
    
51.  [\[51\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0050)
    
    S.T. March, G.D. Scudder
    
    Predictive maintenance: strategic use of IT in manufacturing organizations
    
52.  [\[52\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0051)
    
    X. Liu, T. Yang, J. Pei, H. Liao, E.A. Pohl
    
    Replacement and inventory control for a multi-customer product service system with decreasing replacement costs
    
53.  [\[53\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0052)
    
    P. Engelseth, J.Å. Törnroos, Y. Zhang
    
    Interdependency in coordinating networked maintenance and modification operations
    
54.  [\[54\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0053)
    
    T. Xia, B. Sun, Z. Chen, E. Pan, H. Wang, L. Xi
    
    Opportunistic maintenance policy integrating leasing profit and capacity balancing for serial-parallel leased systems
    
55.  [\[55\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0054)
    
    J. Zhang, X. Zhao, Y. Song, Q. Qiu
    
    Joint optimization of maintenance and spares ordering policy for a use-oriented product-service system with multiple failure modes
    
56.  [\[56\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0055)
    
    R. Sala, M. Bertoni, F. Pirola, G. Pezzotta
    
    Data-based decision-making in maintenance service delivery: the D3M framework
    
57.  [\[57\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0056)
    
    C. Turner, O. Okorie, C. Emmanouilidis, J. Oyekan
    
    Circular production and maintenance of automotive parts: an Internet of Things (IoT) data framework and practice review
    
58.  [\[58\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0057)
    
    K. Petrič, T. Petrič, M. Krisper, V. Rajkovič
    
    User profiling on a pilot digital library with the final result of a new adaptive knowledge management solution
    
59.  [\[59\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0058)
    
    T. Chen, X. Yin, L. Peng, J. Rong, J. Yang, G. Cong
    
    Monitoring and recognizing enterprise public opinion from high-risk users based on user portrait and random forest algorithm
    
60.  [\[60\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0059)
    
    Y. Xu, M. Zhang, X. Xu
    
    A Fast Detection Method of Network Crime Based on User Portrait
    
61.  [\[61\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib70)
    
    S. Liu, L. Wang, X.V. Wang
    
    Sensorless force estimation for industrial robots using disturbance observer and neural learning of friction approximation
    
62.  [\[62\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0060)
    
    Z.Y. Li, G.J. Jiang, H.X. Chen, H. Bin Li, H.H. Sun
    
    Reliability Analysis of Special Vehicle Critical System Using Discrete-Time Bayesian Network
    
63.  [\[63\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0061)
    
    T. Yuge, S. Yanagi
    
    Quantitative analysis of a fault tree with priority AND gates
    
64.  [\[64\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0062)
    
    Y. Wang, Z. Jiang, X. Hu, C. Li
    
    Optimization of reconditioning scheme for remanufacturing of used parts based on failure characteristics
    
65.  [\[65\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0063)
    
    X. Wang, H. Wang, G. Zhao, Z. Liu, H. Wu
    
    ALBERT over Match-LSTM Network for Intelligent Questions Classification in Chinese
    
66.  [\[66\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0064)
    
    Z. Zhang, Y. Lei, J. Xu, X. Mao, X. Chang
    
    TFIDF-FL: localizing faults using term frequency-inverse document frequency and deep learning
    
67.  [\[67\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0065)
    
    T. Peng, L. Liu, W. Zuo
    
    PU text classification enhanced by term frequency-inverse document frequency-improved weighting
    
68.  [\[68\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0066)
    
    D. She, M. Jia, M.G. Pecht
    
    Weighted Entropy Minimization Based Deep Conditional Adversarial Diagnosis Approach under Variable Working Conditions
    
69.  [\[69\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0067)
    
    M.P. Limongelli, P.F. Giordano
    
    Vibration-based damage indicators: a comparison based on information entropy
    
70.  [\[70\]](https://www.sciencedirect.com/science/article/pii/S0736584522001673#bbib0068)
    
    T. Xiao, Y. Xu, H. Yu
    
    Research on Obstacle Detection Method of Urban Rail Transit Based on Multisensor Technology
    

## Cited by (3)

© 2022 The Author(s). Published by Elsevier Ltd.
