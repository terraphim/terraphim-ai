---
created: 2023-09-21T15:52:02 (UTC +01:00)
tags: []
source: https://www.sciencedirect.com/science/article/pii/S2210670723001440
author: Z.T. Ai, A.K. Melikov
---

# Intelligent operation, maintenance, and control system for public building: Towards infection risk mitigation and energy efficiency - ScienceDirect

> ## Excerpt
> During the post-COVID-19 era, it is important but challenging to synchronously mitigate the infection risk and optimize the energy savings in public b…

---
[![Elsevier](https://sdfestaticassets-eu-west-1.sciencedirectassets.com/prod/c5ec5024630bc984ae859b0b2315edad4a342b5a/image/elsevier-non-solus.png)](https://www.sciencedirect.com/journal/sustainable-cities-and-society "Go to Sustainable Cities and Society on ScienceDirect")

[![Sustainable Cities and Society](https://ars.els-cdn.com/content/image/1-s2.0-S2210670723X00043-cov150h.gif)](https://www.sciencedirect.com/journal/sustainable-cities-and-society/vol/93/suppl/C)

[https://doi.org/10.1016/j.scs.2023.104533](https://doi.org/10.1016/j.scs.2023.104533 "Persistent link using digital object identifier")[Get rights and content](https://s100.copyright.com/AppDispatchServlet?publisherName=ELS&contentID=S2210670723001440&orderBeanReset=true)

## Highlights

-   •
    
    Intelligent operation, maintenance, and control system was built for public building.
    
-   •
    
    LLM-based ANN rapidly predicted CO<sub>2</sub> concentration field by combining limited sensors.
    
-   •
    
    Ventilation performance of energy savings and infection prevention were optimized.
    
-   •
    
    Removal efficiency was improved by optimizing layouts of air purification system.
    

## Abstract

During the post-COVID-19 era, it is important but challenging to synchronously mitigate the infection risk and optimize the energy savings in public buildings. While, ineffective control of ventilation and purification systems can result in increased energy consumption and cross-contamination. This paper is to develop intelligent operation, maintenance, and control systems by coupling intelligent ventilation and air purification systems (negative ion generators). Optimal deployment of sensors is determined by Fuzzy C-mean (FCM), based on which CO<sub>2</sub> concentration fields are rapidly predicted by combing the artificial neural network (ANN) and self-adaptive low-dimensional linear model (LLM). Negative oxygen ion and particle concentrations are simulated with different numbers of negative ion generators. Optimal ventilation rates and number of negative ion generators are decided. A visualization platform is established to display the effects of ventilation control, epidemic prevention, and pollutant removal. The rapid prediction error of LLM-based ANN for CO<sub>2</sub> concentration was below 10% compared with the simulation. Fast decision reduced CO<sub>2</sub> concentration below 1000 ppm, infection risk below 1.5%, and energy consumption by 27.4%. The largest removal efficiency was 81% when number of negative ion generators was 10. This work can promote intelligent operation, maintenance, and control systems considering infection prevention and energy sustainability.

-   [Previous](https://www.sciencedirect.com/science/article/pii/S2210670723001312)
-   [Next](https://www.sciencedirect.com/science/article/pii/S2210670723001294)

## Keywords

Public building environment

Intelligent operation, maintenance and control system

Ventilation

Air purification

Infection risk

Energy efficiency

## 1\. Introduction

With the global urbanization, the building area is rapidly increasing, especially for public buildings ([Huang et al., 2022](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bib0018)). Enormous energy and resource consumption are accompanied by the rapid development of the building industry ([Su et al., 2022](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bib0041)). Building energy consumption has accounted for around 40% of global energy consumption, which has been the key to energy conservation and low-carbon development ([Chen et al., 2020](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bib0004)). The regulation of energy consumption in public buildings is conducive to the sustainable construction and development of urban cities ([Ma et al., 2021](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bib0025)). Public buildings can improve the building energy efficiency through operation, maintenance, and control of energy-intensive units, such as ventilation systems ([Gupta et al., 2017](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bib0016)). While, a major task of the ventilation system is to provide the adequate ventilation rates to dilute the indoor pollutants. If energy efficiency is improved at the expense of indoor environmental quality, it can potentially lead to a series of emergent incidents of building environment, such as transmission of infectious disease and transfer of outdoor pollutants (naturally/mechanically), which can seriously threaten the building occupants’ health and cause the incalculable loss ([Rohde et al., 2020](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bib0037)). Therefore, indoor environment improvement and energy conservation are both core objectives of operation, maintenance, and control system for public buildings. Particularly, global pandemic of coronavirus disease 2019 (COVID-19) poses a challenge to the regulation of safe and healthy environment, and energy-saving building ([Ding et al., 2020](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bib0007)).

The essential task of developing operation, maintenance, and control system for public building environment is to reduce the pollutant concentration (e.g., CO<sub>2</sub>, particulate matter), infection risk, and energy consumption. Intelligent ventilation and air purification are both important to control the pollutant exposure level ([Feng et al., 2021](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bib0010); [Zhu et al., 2022](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bib0048)), which provide the key technologies for the operation, maintenance, and control of indoor environment.

### 1.1. Intelligent ventilation system

For public buildings with the high density and mobility of people, indoor pollutants are generated by a number of sources (e.g., indoor occupants, furnishing and building material, cosmetic and cleaning products, and outdoor sources), which can present the non-uniform, dynamic, and complex distribution characteristics ([Feng et al., 2020](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bib0009)). Using the traditional ventilation systems with constant supply air parameters may result in the local accumulation of pollutants, cross-infection, and energy waste ([Ren et al., 2022](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bib0035)). To address these problems, intelligent control of building ventilation systems is significant based on non-uniform and dynamic pollutant distribution, namely intelligent ventilation system. Its essence is (i) _how to rapidly obtain indoor pollutant distributions,_ and (ii) _how to implement fast decision and intelligent control?_

Traditional prediction methods for indoor environment such as computational fluid dynamics (CFD) require a long computation time and cannot be used directly for online control. Using a number of monitoring sensors to obtain the non-uniform indoor environment is usually accompanied by data duplication and high costs ([Kumar et al., 2016](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bib0019)). It is urgent to develop rapid prediction models for non-uniform and dynamic distributions of pollutants, by using the scientific deployment strategy of limited monitoring sensors as input boundaries. [Ren and Cao (2019)](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bib0036) incorporated monitoring techniques into the low-dimensional linear model (LLM) and artificial neural network (ANN) to predict pollutant concentration efficiently, and found that well-deployed sensors could provide the satisfying inputs for rapid prediction. [Cao et al. (2020)](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bib0003) developed a systematic method of sensor deployment for efficient prediction of indoor environment using Fuzzy C-means (FCM) clustering method. However, these studies did not apply the deployment strategy of limited sensors or rapid prediction models to real-life ventilation systems in buildings.

The decision of conventional ventilation systems commonly depends on a single-point or limited monitoring data for direct evaluation and control, which cannot effectively represent the non-uniform distribution of indoor pollutant, potentially leading to ineffective decision making ([Merema et al., 2018](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bib0026)). Thus, the aim of fast decision for ventilation systems should be implementing the scientific evaluation and control (combined with controller) based on the rapid prediction of pollutant distribution, to effectively reduce pollutant exposure level and infection risk, and also minimize energy consumption. [Ren and Cao (2020)](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bib0032) developed a faster-than-real-time prediction model (low-dimensional linear ventilation model, LLVM) to predict the non-uniform pollutant concentration fields, and evaluated the ventilation performance with energy efficiency and pollutant level reduced by 43% and 28% after the decision when compared with that before the decision, respectively. However, no earlier studies have synthetically considered the feasibility of fast decision and control systems (strategies) in real building application.

Generally, intelligent ventilation system should consist of “limited monitoring system”, “rapid prediction model”, and “fast decision making”. Therefore, it is of necessity to explore the application of intelligent ventilation system to the actual public building environment.

### 1.2. Air purification system

Regarding the air purification system, air filters or ultraviolet (UV) lamps are usually used to remove indoor pollutant ([Feng et al., 2021](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bib0011)). High efficiency particulate air filter (HEPA) can approach to about 100% of removal efficiency for particles, yet some shortcomings still be found. (1) The accumulation of particles can result in high pressure drop, which would increase the energy consumption and replacement costs for fiber filters ([Li et al., 2019](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bib0021)). (2) Bacteria and viruses can multiply in fiber filter with the re-atomization and bio-release potential, which can cause secondary contamination and health risk ([Nakpan et al., 2019](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bib0027)). UV lamps have the function of disinfection to inactivate pollutants such as bacteria and viruses. However, UV lamps almost have no purification ability to remove the particles ([Fischer et al., 2020](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bib0012)). Moreover, air filters and UV lamps are ineffective against the pollutants that settle on walls, and are usually applied to indoor microenvironments ([D'Orazio & D'Alessandro, 2020](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bib0008)). To this end, portable air purification system, such as negative ion generator, is developed for effective removal of indoor pollutants (especially particles).

The negative ion generator can release a large number of negative oxygen ions through carbon fiber electrode ([Pushpawela et al., 2017](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bib0030)). Negative oxygen ions can electrically charge the particles, causing them to either repel each other or remove them by a deposition process ([Pushpawela et al., 2017](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bib0030)). Considering the spatial accessibility of negative oxygen ions, negative ion generators could effectively remove the pollutants from indoor air and indoor environment surfaces. [Pushpawela et al. (2017)](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bib0030) measured the efficiency of a negative ion generator in removing ultrafine particles from an indoor chamber, and found that 70% of particles were removed within 15 min. [Grabarczyk (2001)](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bib0015) used ionizers in an unventilated and unoccupied room, and found that the particle concentrations reduced by up to two orders of magnitude after 2 h for the particles in the size range of 0.3–2.5 μm. However, the major drawback of negative ion generators is the potential harmful effect on building occupant's health ([Ren et al., 2022](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bib0034)). Generally, it is suggested to use negative ion generators without the appearance of occupants, considering their side effects on health ([Ren et al., 2022](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bib0034)).

In buildings, the air purification system's layout strategy (e.g., location and number) has been optimally designed to efficiently remove indoor pollutants. For example, [Yu et al. (2017)](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bib0044) analyzed the pollutant concentration after installing air purification device, and evaluated their optimal locations. [Shiue et al. (2011)](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bib0040) evaluated the particle removal efficiencies at different installation heights of negative ion generator in a closed room, showing the maximum particle removal efficiency at the installation height of 60 cm from the floor. Most studies considered the effect of independently using the air purification system, yet usually ignored the impact of indoor ventilation on the air purification efficiency. For public buildings, investigating the coupling performance of intelligent ventilation and air purification systems is beneficial to operation, maintenance, and control for safe and healthy environments, and energy-efficient building.

### 1.3. Research framework

This study aims to develop the integrated and intelligent operation, maintenance, and control system for safe and healthy indoor environment, and energy efficient public buildings, through coupling intelligent ventilation and air purification system. This work systematically integrates the limited monitoring technology, rapid prediction model, and fast decision making into a real building scenario.

The research framework is presented in [Fig. 1](https://www.sciencedirect.com/science/article/pii/S2210670723001440#fig0001). First, different layout strategies (different numbers and locations) of monitoring sensors are determined, and the experiment is performed to validate the simulation model. Second, rapid prediction of CO<sub>2</sub> concentration fields is achieved and validated by CFD simulation, based on which the optimal deployment of sensors is evaluated. Third, the optimal supply air velocity and the number of negative ion generators are evaluated by using fast decision. Fourth, online regulation of intelligent ventilation and air purification system is achieved based on optimal evaluation, and a visualization platform of intelligent operation, maintenance, and control system is constructed to show the actual implementation effectiveness.

![Fig 1](https://ars.els-cdn.com/content/image/1-s2.0-S2210670723001440-gr1.jpg)

1.  [Download : Download high-res image (677KB)](https://ars.els-cdn.com/content/image/1-s2.0-S2210670723001440-gr1_lrg.jpg "Download high-res image (677KB)")
2.  [Download : Download full-size image](https://ars.els-cdn.com/content/image/1-s2.0-S2210670723001440-gr1.jpg "Download full-size image")

Fig. 1. Research framework of this work. (For interpretation of the references to color in this figure legend, the reader is referred to the web version of this article.)

## 2\. Materials and methods

This study uses a numerical simulation method to model the velocity, pollutant concentration fields, and distributions of negative oxygen ions. For intelligent ventilation system, online monitoring and experiment are first carried out. Fuzzy C-means (FCM) is used to determine the deployment strategy of limited sensors. The rapid prediction models, including low-dimensional linear model (LLM) and artificial neural network (ANN, using limited monitoring data as inputs), are used to predict the non-uniform pollutant concentration fields rapidly. The fast decisions for optimal supply air velocity are proposed by evaluating the ventilation performance based on analytic hierarchy process (AHP). For air purification system, different layout strategies of negative ion generators are defined, and the fast decision based on the removal efficiency is proposed for optimal number of negative ion generators. The framework of intelligent operation, maintenance, and control system is finally developed.

### 2.1. Building model setup

The front hall of the International Convention and Exhibition Center (a public building) in Nanjing is selected for the intelligent operation, maintenance, and control of indoor environment. [Fig. 2](https://www.sciencedirect.com/science/article/pii/S2210670723001440#fig0002) shows the photo and geometry of the front hall, with a floor area of 292 m<sup>2</sup> and a height of 15 m. The hall is equipped with a variable air volume system for the intelligent ventilation, with an outdoor air volume (OAV) accounting for 32% of the total flow rate. The intelligent ventilation system is mainly used to control indoor pollutant level (e.g., CO<sub>2</sub>). The ventilation mode is mixing ventilation (MV) with nine circular supply air inlets and one louvered return air outlet placed in the ceiling. The radius of an inlet is 0.2 m, and the size of an outlet is 2.00 m × 0.96 m. The number of people (_N<sub>p</sub>_) to be accommodated in the hall is between 0 and 100. According to an OAV of 30 m<sup>3</sup>/h per person ([GB50736-2012, 2012](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bib0014)), the maximum supply air volume is set as 9300 m<sup>3</sup>/h, with an inlet velocity (_V<sub>in</sub>_) corresponding to 2.3 m/s. The _V<sub>in</sub>_ values of 0.7 and 1.5 m/s are also considered, and the supply air temperature is constant at 24 °C (summer condition).

![Fig 2](https://ars.els-cdn.com/content/image/1-s2.0-S2210670723001440-gr2.jpg)

1.  [Download : Download high-res image (373KB)](https://ars.els-cdn.com/content/image/1-s2.0-S2210670723001440-gr2_lrg.jpg "Download high-res image (373KB)")
2.  [Download : Download full-size image](https://ars.els-cdn.com/content/image/1-s2.0-S2210670723001440-gr2.jpg "Download full-size image")

Fig. 2. (a) Photo and (b) geometry of the front hall in the International Convention and Exhibition Center. (For interpretation of the references to color in this figure legend, the reader is referred to the web version of this article.)

The air purification system in the front hall is used to enhance the removal effects of pollutants (i.e., particles), and it is further combined with intelligent ventilation system. In this study, negative ion generator (KT03-A01) is applied to remove particles, with the size of 98 mm (length) × 72 mm (width) × 225 mm (height). A number of negative ion generators are arranged along the walls with a height of 0.5 m from the floor. The setup of negative ion generator is detailed in [Section 2.4](https://www.sciencedirect.com/science/article/pii/S2210670723001440#sec0012).

### 2.2. Numerical simulation

CFD is used to simulate the airflow and distribution of pollutants (e.g., CO<sub>2</sub> and particles), to illustrate the effectiveness of integrated intelligent ventilation and air purification systems. All simulations are performed at steady-state assuming incompressible using ANSYS FLUENT 16.0. The Reynolds-averaged Navier-Stokes (RANS) method is used, and the _Re_\-Normalization Group (RNG) k-ε model is used for the turbulence modeling because of its great performance of airflow simulation ([Ren et al., 2022](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bib0033)). Based on the airflow simulation, the CO<sub>2</sub> concentration is simulated by the species transport equation.(1)$<math><mrow is="true"><mi is="true">∇</mi><mrow is="true"><mo stretchy="true" is="true">(</mo><mrow is="true"><mi is="true">ρ</mi><mi mathvariant="bold-italic" is="true">u</mi><mi is="true">C</mi></mrow><mo stretchy="true" is="true">)</mo></mrow><mo linebreak="goodbreak" is="true">=</mo><mi is="true">∇</mi><mrow is="true"><mo stretchy="true" is="true">[</mo><mrow is="true"><mi is="true">ρ</mi><mrow is="true"><mo stretchy="true" is="true">(</mo><mrow is="true"><mi is="true">D</mi><mo linebreak="badbreak" is="true">+</mo><msub is="true"><mi is="true">υ</mi><mi is="true">C</mi></msub></mrow><mo stretchy="true" is="true">)</mo></mrow><mi is="true">∇</mi><mi is="true">C</mi></mrow><mo stretchy="true" is="true">]</mo></mrow><mo linebreak="goodbreak" is="true">+</mo><msub is="true"><mi is="true">S</mi><mi is="true">C</mi></msub></mrow></math>$where, _C_ is the species concentration for CO<sub>2</sub> (ppm); _ρ_ is the density (kg/m<sup>3</sup>); **_u_** is the velocity (m/s); _D_ is the diffusion coefficient (m<sup>2</sup>/s); $<math><msub is="true"><mi is="true">υ</mi><mi is="true">C</mi></msub></math>$ is the turbulent diffusion coefficient (m<sup>2</sup>/s); and _S<sub>C</sub>_ is the source term. For particulate pollutant, fine particles (i.e., PM<sub>2.5</sub>, typical particle size existing in indoor environment) are considered in this study. Considering that modeling particles as gaseous pollutants (continuous phase) is acceptable when the particle diameter is below 3.5 μm according to [Ai and Melikov (2018](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bib0001)), the particle concentration is simulated using a user-defined scalar (UDS) function by solving the species transport equation based on the airflow simulation, shown in [Eq. (2)](https://www.sciencedirect.com/science/article/pii/S2210670723001440#eqn0002). The simulations of CO<sub>2</sub> and particle are assumed decoupled in this study.(2)$<math><mrow is="true"><mi is="true">∇</mi><mrow is="true"><mo stretchy="true" is="true">[</mo><mrow is="true"><mrow is="true"><mo stretchy="true" is="true">(</mo><mrow is="true"><mi mathvariant="bold-italic" is="true">u</mi><mo linebreak="badbreak" is="true">+</mo><msub is="true"><mi is="true">u</mi><mi is="true">s</mi></msub></mrow><mo stretchy="true" is="true">)</mo></mrow><msub is="true"><mi is="true">C</mi><mi is="true">P</mi></msub></mrow><mo stretchy="true" is="true">]</mo></mrow><mo linebreak="goodbreak" is="true">=</mo><mi is="true">∇</mi><mrow is="true"><mo stretchy="true" is="true">[</mo><mrow is="true"><mrow is="true"><mo stretchy="true" is="true">(</mo><mrow is="true"><mi is="true">D</mi><mo linebreak="badbreak" is="true">+</mo><msub is="true"><mrow is="true"><mi is="true">ε</mi></mrow><mi is="true">p</mi></msub></mrow><mo stretchy="true" is="true">)</mo></mrow><mi is="true">∇</mi><msub is="true"><mi is="true">C</mi><mi is="true">P</mi></msub></mrow><mo stretchy="true" is="true">]</mo></mrow><mo linebreak="goodbreak" is="true">−</mo><mi is="true">A</mi><mi is="true">n</mi><msub is="true"><mi is="true">C</mi><mi is="true">P</mi></msub></mrow></math>$where, _C<sub>p</sub>_ is the particle concentration (#/m<sup>3</sup>); _u<sub>s</sub>_ is the settling velocity of particles (m/s); _ε<sub>p</sub>_ is the particle eddy diffusivity (m<sup>2</sup>/s); _n_ is the number of negative oxygen ions (#/cm<sup>3</sup>); and _A_ is the removal coefficient of negative oxygen ions, which is equal to 1 × 10<sup>−7</sup> cm<sup>3</sup>/#. An oxygen ion is assumed to carry a negative charge. Potential and electrical fields are solved by Poisson and Gaussian equations, respectively. Negative oxygen ion is simulated by continuous phase model, as follows.(3)$<math><mrow is="true"><mrow is="true"><mo stretchy="true" is="true">(</mo><mrow is="true"><mi mathvariant="bold-italic" is="true">u</mi><mo linebreak="badbreak" is="true">+</mo><msub is="true"><mi is="true">μ</mi><mi is="true">i</mi></msub><mo is="true">·</mo><mi is="true">E</mi></mrow><mo stretchy="true" is="true">)</mo></mrow><mspace width="0.33em" is="true"></mspace><mi is="true">∇</mi><mi is="true">n</mi><mo linebreak="goodbreak" is="true">=</mo><mi is="true">D</mi><msup is="true"><mrow is="true"><mi is="true">∇</mi></mrow><mn is="true">2</mn></msup><mi is="true">n</mi></mrow></math>$where, _μ<sub>i</sub>_ is the negative oxygen ion mobility (m<sup>2</sup>/V·s); and _E_ is the electric field (V/m).

In CFD modeling, the supply inlets are defined as velocity-inlet with a uniform velocity profile, and the return outlet is set as outflow. The walls are defined as non-slip walls and insulated. In the CO<sub>2</sub> simulation, CO<sub>2</sub> release rate per person is 16.8 g/h ([Chow, 2002](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bib0005)), and _N<sub>p</sub>_ in the hall is defined as a constant of 20, 40, 60, 80, 100, and 120. The CO<sub>2</sub> source is defined as a surface source (covering a surface area of 292 m<sup>2</sup>) located at a height of 1.7 m (the height of breathing plane). The CO<sub>2</sub> release is assumed uniform on this surface, with different CO<sub>2</sub> source densities corresponding to different values of _N<sub>p</sub>_. The CO<sub>2</sub> source is compiled by using user-defined function (UDF). In the simulation of particles, the total particle concentration (_C<sub>0</sub>_) at the inlets is defined as 11,000 #/cm<sup>3</sup> (measured by particle counter), and indoor particle concentrations are normalized by _C<sub>p</sub>_/_C<sub>0</sub>_. The modeling parameters of releasing negative oxygen ions from negative ion generators are described in [Section 2.4](https://www.sciencedirect.com/science/article/pii/S2210670723001440#sec0012). Grid independence analysis is performed considering coarse, medium, and fine grids with the scenario of _V<sub>in</sub>_ = 0.7 m/s, and _N<sub>p</sub>_ = 20, which can be referred to [Section 3.2](https://www.sciencedirect.com/science/article/pii/S2210670723001440#sec0018).

### 2.3. Intelligent ventilation system

#### 2.3.1. Online monitoring and experiment

Developing intelligent ventilation needs the online monitoring of indoor environment to provide input boundaries for rapid prediction and fast decision. Considering the non-uniform distributions of indoor environment, a clustering algorithm of Fuzzy C-means (FCM) is applied to determine the deployment strategy of limited sensors, which can greatly improve the monitoring effectiveness, and reduce the monitoring cost (dependent on the number of sensors) ([Cao et al., 2020](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bib0003)).

FCM is one of the widely applied and excellent unsupervised machine learning methods ([Cao et al., 2020](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bib0003)). It obtains the degrees of membership for each sample data to the cluster centers through optimizing the objective function, thus determining the category of sample data to automatically classify the sample set. In this study, a series of CO<sub>2</sub> concentration and velocity data (by considering different _V<sub>in</sub>_ and _N<sub>p</sub>_) are acquired by CFD simulation, and these data are normalized. The normalized concentration _C\*_ and velocity _U\*_ are clustered by minimizing the objective function ([Cao et al., 2020](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bib0003)). More details of FCM are shown in **Appendix A**. Next, cluster centers are defined as locations of sensors (potentially with the adjustment in engineering installation). The deployment of limited sensors in the front hall is shown in [Section 3.1](https://www.sciencedirect.com/science/article/pii/S2210670723001440#sec0017) (with the number of sensors depending on the cluster number). The rapid prediction of non-uniform pollutant concentration fields can be achieved by inputting limited monitoring data. The optimal strategy (locations and number) is determined by comparing the rapid prediction errors under different layout strategies of sensors.

The sensors are also used for online monitoring of other environmental parameters, including temperature, velocity, PM<sub>2.5</sub>, total volatile organic compounds (TVOC), and relative humidity (RH). [Fig. 3](https://www.sciencedirect.com/science/article/pii/S2210670723001440#fig0003) displays the photo of a monitoring sensor, consisting of a particle sensor module (BITSHEEP PMS5003, ± 10%), a CO<sub>2</sub> sensor module (SenseAir S8, ± 3%), a velocity transducer (W410C2, ± 0.02 m/s), a TVOC sensor module (AMS IAQ-CORE C, ± 5%), a temperature and humidity sensor module (SENSIRION SHT20, ± 0.5 °C and ± 4% for RH), and a wireless transmission module (WH-LTE-7S1). TVOC is consisted of benzene series, organic chlorides, freon series, organic ketones, amines, alcohols, ethers, esters, acids and petroleum hydrocarbon compounds, which are usually emitted from chemicals, cleaning agents, building materials, and furnishings ([Zhang & Zhang, 2007](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bib0045)). Thus, particular attention should be paid on the determination of concentrations of TVOC. The online monitoring sensors can transmit the data to the web server by the LTE Cat-1 and GPRS networks, setting the interval of recording and transmitting the data as 30 min. By collecting the video data from the surveillance cameras in the hall and adopting the YOLO (You Only Look Once) algorithm ([Wang et al., 2021](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bib0042)), real-time detection of _N<sub>p</sub>_ is achieved. The schematic of YOLO algorithm can be found in [Wang et al. (2021](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bib0042)).

![Fig 3](https://ars.els-cdn.com/content/image/1-s2.0-S2210670723001440-gr3.jpg)

1.  [Download : Download high-res image (450KB)](https://ars.els-cdn.com/content/image/1-s2.0-S2210670723001440-gr3_lrg.jpg "Download high-res image (450KB)")
2.  [Download : Download full-size image](https://ars.els-cdn.com/content/image/1-s2.0-S2210670723001440-gr3.jpg "Download full-size image")

Fig. 3. Photo of monitoring sensor in the front hall. (For interpretation of the references to color in this figure legend, the reader is referred to the web version of this article.)

Experimental measurement is carried out using the monitoring sensors to verify the simulation model. Three _V<sub>in</sub>_ are considered in the experiment, and the measured parameters includes the velocity and CO<sub>2</sub> concentration. The measurement points are the locations of 3 monitoring sensors, shown in [Section 3.1](https://www.sciencedirect.com/science/article/pii/S2210670723001440#sec0017). For _V<sub>in</sub>_, the measurement duration is 9 h, and _N<sub>p</sub>_ is recorded with the interval of 30 min. The inlet CO<sub>2</sub> (background) concentration is measured to be 400 ppm, which is used in the simulation.

#### 2.3.2. Rapid prediction models

Rapid prediction models are used to predict the non-uniform distributions of indoor environment for fast decision and control of intelligent ventilation system. Low-dimensional linear model (LLM) is constructed for rapid prediction of CO<sub>2</sub> concentration, including low-dimensional model (LM, reducing the data dimensionality and storage capacity of the database) and linear model (reducing the construction cost of the database) ([Ren & Cao, 2019](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bib0031)). LM consists of uniform LM and self-adaptive non-uniform LM. Linear model is used to rapidly expand the database based on LM.

[Fig. 4](https://www.sciencedirect.com/science/article/pii/S2210670723001440#fig0004) illustrates the schematic diagram for uniform LM and self-adaptive non-uniform LM. To reduce the system's dimensionality, “sub-layer division and recomposition” algorithm is proposed to determine the division point set _P<sub>d</sub>_ of _d_ (_d_ = X_,_ Y, and Z) direction \[including one zero point, one full-length point, and (_N<sub>d</sub>_ −1) middle points; _N<sub>d</sub>_ is the division number of _d_ direction\]. Details of LM steps are shown in **Appendix B**. The _N<sub>d</sub>_ in direction _d_ of non-uniform LM is same as that of uniform LM. In this study, the performance of two LMs is compared, and the calculation method for discretization error _ε_ is also defined, as shown in Eq. B5 in **Appendix B**.

![Fig 4](https://ars.els-cdn.com/content/image/1-s2.0-S2210670723001440-gr4.jpg)

1.  [Download : Download high-res image (819KB)](https://ars.els-cdn.com/content/image/1-s2.0-S2210670723001440-gr4_lrg.jpg "Download high-res image (819KB)")
2.  [Download : Download full-size image](https://ars.els-cdn.com/content/image/1-s2.0-S2210670723001440-gr4.jpg "Download full-size image")

Fig. 4. Schematic diagram of low-dimensional model (LM) consisting of uniform LM and self-adaptive non-uniform LM. (For interpretation of the references to color in this figure legend, the reader is referred to the web version of this article.)

The linear model is used to superimpose linearly the scalar fields generated by any source within the steady-state airflow ([Ren & Cao, 2019](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bib0031)). For example, when multiple pollutant sources exist in the room, indoor pollutant concentration fields generated by multiple sources are equal to the linear superpositions of those generated by a single source. Compared with the simulation, linear model can effectively reduce the database construction cost used for prediction, and improve the efficiency of rapid prediction. The discretization error _ε_ is also used to validate the LLM.

Based on the LLM, artificial neural network (ANN) model with autonomous learning function is used to improve the rapid prediction effect of indoor environment, responding to limited monitoring data ([Zhang & You, 2017](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bib0046)). In this study, radial basis function (RBF) network with better approximation and learning ability is applied to train and predict low-dimensional and non-uniform pollutant concentration fields. We construct a database for ANN training with monitored CO<sub>2</sub> concentration and velocity under different values of _V<sub>in</sub>_ and _N<sub>p</sub>_ as inputs, and the low-dimensional CO<sub>2</sub> concentration fields as outputs. Firstly, the output data in the initial database are derived from the cases with _V<sub>in</sub>_ of 0.7, 1.5, and 2.3 m/s and _N<sub>p</sub>_ of 20, 40, 60, 80, 100, and 120, which are high-resolution data from CFD simulation. Then, the recomposition of low-dimensional database is obtained by LM. Finally, the database is expanded by the linear model to obtain the low-dimensional data with the scenarios of _V<sub>in</sub>_ = 0.7, 1.5, and 2.3 m/s, and _N<sub>p</sub>_ = 10, 30, 70, 90, and 110. After training the ANN, the simulation case with _V<sub>in</sub>_ = 0.7 m/s and _N<sub>p</sub>_ = 50 is applied to validate the rapid prediction model with monitoring data from different layouts of sensor as inputs. The prediction errors are calculated as follows to determine the optimal layout.(4)$<math><mrow is="true"><mtext is="true">MAPE</mtext><mo linebreak="goodbreak" is="true">=</mo><mfrac is="true"><mrow is="true"><msubsup is="true"><mo is="true">∑</mo><mrow is="true"><mi is="true">S</mi><mo is="true">=</mo><mn is="true">1</mn></mrow><mi is="true">N</mi></msubsup><msub is="true"><mstyle mathvariant="normal" is="true"><mi is="true">Ω</mi></mstyle><mi is="true">S</mi></msub><mrow is="true"><mo stretchy="true" is="true">|</mo><mrow is="true"><msub is="true"><mi is="true">C</mi><mrow is="true"><mi is="true">C</mi><mi is="true">F</mi><mi is="true">D</mi><mo is="true">−</mo><mi is="true">L</mi><mi is="true">L</mi><mi is="true">M</mi><mo is="true">,</mo><mi is="true">S</mi></mrow></msub><mo is="true">−</mo><msub is="true"><mi is="true">C</mi><mrow is="true"><mi is="true">A</mi><mi is="true">N</mi><mi is="true">N</mi><mo is="true">,</mo><mi is="true">S</mi></mrow></msub></mrow><mo stretchy="true" is="true">|</mo></mrow><mo is="true">/</mo><msub is="true"><mi is="true">C</mi><mrow is="true"><mi is="true">A</mi><mi is="true">N</mi><mi is="true">N</mi><mo is="true">,</mo><mi is="true">S</mi></mrow></msub></mrow><mi is="true">N</mi></mfrac><mo linebreak="goodbreak" is="true">×</mo><mspace width="0.33em" is="true"></mspace><mn is="true">100</mn><mo is="true">%</mo></mrow></math>$where, MAPE is the mean absolute percentage error for rapid prediction model; $<math><msub is="true"><mstyle mathvariant="normal" is="true"><mi is="true">Ω</mi></mstyle><mi is="true">S</mi></msub></math>$ is the volume of zone _S_; $<math><msub is="true"><mi is="true">C</mi><mrow is="true"><mi is="true">C</mi><mi is="true">F</mi><mi is="true">D</mi><mo is="true">−</mo><mi is="true">L</mi><mi is="true">L</mi><mi is="true">M</mi><mo is="true">,</mo><mi is="true">S</mi></mrow></msub></math>$ represents the low-dimensional CO<sub>2</sub> concentrations by CFD-based LLM in zone _S_; $<math><msub is="true"><mi is="true">C</mi><mrow is="true"><mi is="true">A</mi><mi is="true">N</mi><mi is="true">N</mi><mo is="true">,</mo><mi is="true">S</mi></mrow></msub></math>$ is the predicted CO<sub>2</sub> concentrations in zone _S; S_ is the index of divided zone; and _N_ is the total number of zone _S_.

#### 2.3.3. Fast decision for controlling ventilation system

The fast decision of intelligent ventilation systems aims to evaluate the ventilation performance, and then determine the optimal ventilation rate (or _V<sub>in</sub>_). The evaluation of ventilation performance is done in terms of indoor air quality (IAQ), epidemic prevention, and ventilation energy consumption. IAQ is evaluated by CO<sub>2</sub> concentrations, which is an effective bio-proxy for indicating IAQ ([Lu et al., 2022](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bib0023)). Besides, CO<sub>2</sub> can be used as a good proxy for pathogen concentration and infection risk since it is co-exhaled with the aerosols containing pathogens by infectors ([Li & Cai, 2022](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bib0020)), which is described in [Eq. (5)](https://www.sciencedirect.com/science/article/pii/S2210670723001440#eqn0005). CO<sub>2</sub> monitoring inside enclosed environment is revealing to be a practical warning to prevent airborne transmission ([Palmisani et al., 2021](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bib0028)). Total IAQ is then evaluated comprehensively based on PM<sub>2.5</sub>, TVOC, temperature, and humidity, since that IAQ is also particularly relative to the determination of TVOC, PM<sub>2.5</sub> (related to outdoor infiltration and indoor particle resuspension), and other parameters such as temperature and relative humidity ([Paterson et al., 2021](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bib0029)). Through measuring the concentrations of TVOC and PM<sub>2.5</sub>, indoor health risks (such as asthma and other bronchial diseases) and the harmful impact of giving rise to irritant odors, causing the body immune level disorder, and affecting the central nervous system can be also forewarned ([Paterson et al., 2021](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bib0029)). For the epidemic prevention, prevention effect and infection risk are assessed based on the airborne pollutant level, and then to analyze the epidemic prevention level. Ventilation energy consumption is calculated based on the cooling loads. The fast decision of the optimal _V<sub>in</sub>_ is conducted by integrated evaluation of pollutant concentration, infection risk, and energy consumption.

This study focuses on the average CO<sub>2</sub> concentration $<math><mrow is="true"><mo is="true">〈</mo><mi is="true">C</mi><mo is="true">〉</mo></mrow></math>$ (ppm) in the breathing zone (_Z_ ≤ 1.7 m) as an evaluation index for IAQ. The criterion is defined as: “$<math><mrow is="true"><mo is="true">〈</mo><mi is="true">C</mi><mo is="true">〉</mo></mrow></math>$ ≤ 800 ppm” for “Excellent”, “800 ppm < $<math><mrow is="true"><mo is="true">〈</mo><mi is="true">C</mi><mo is="true">〉</mo></mrow></math>$ ≤ 1000 ppm” for “Good”, and “$<math><mrow is="true"><mo is="true">〈</mo><mi is="true">C</mi><mo is="true">〉</mo></mrow></math>$ > 1000 ppm” for “Poor” according to [GB/T18883-2022](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bib0013). CO<sub>2</sub> concentration limit of 1000 ppm is commonly recommended in different countries standards for the management of generic IAQ concerns ([Lu et al., 2022](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bib0024)). For total IAQ, the evaluation criteria for PM<sub>2.5</sub>, TVOC, temperature, and RH are displayed in [Table 1](https://www.sciencedirect.com/science/article/pii/S2210670723001440#tbl0001) according to [GB/T18883-2022](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bib0013). The threshold of PM<sub>2.5</sub> is 50 μg/m<sup>3</sup>, which is consistent with daily average limits of PM<sub>2.5</sub> concentration proposed by the World Health Organization (WHO) ([Xue et al., 2020](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bib0043)). The threshold of TVOC is set 0.6 mg/m<sup>3</sup>, which almost corresponds to the limit of 0.5 mg/m<sup>3</sup> established in different IAQ indices such as Indoor Environment Index (IEI) in Taiwan, and Indoor Air Quality Certification (IAQC) proposed by Indoor Air Quality Management Group in Hong Kong ([Zhang et al., 2020](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bib0047)). The criteria for indoor temperature and RH are also within the comfort range proposed by the American Society of Heating Refrigerating and Air conditioning Engineer (ASHRAE) ([Alghamdi et al., 2023](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bib0002)). The evaluation results of total IAQ are given by calculating the percentage of at least “Good” among four parameters. Note that the evaluation criteria for (total) IAQ used in this work are more applicable in the studied building as shown in [Fig. 2](https://www.sciencedirect.com/science/article/pii/S2210670723001440#fig0002), which is located in Nanjing, China. The criteria can be adapted when intelligent ventilation control system is applied in other countries or regions. It is better to develop reliable values of criteria to improve the performance of intelligent ventilation control, which is discussed in [Section 4](https://www.sciencedirect.com/science/article/pii/S2210670723001440#sec0025).

Table 1. Evaluation criteria for total IAQ.

| Empty Cell | PM<sub>2.5</sub> (μg/m<sup>3</sup>) | TVOC (mg/m<sup>3</sup>) | Temperature (°C) | RH (%) |
| --- | --- | --- | --- | --- |
| Excellent | ≤ 25 | ≤ 0.3 |  |  |
| Good | 25–50 | 0.3–0.6 | 24–28 | 40–60 |
| Poor | \> 50 | \> 0.6 |  |  |

Airborne infection risk _R<sub>inf</sub>_ (%) is evaluated by the revisited Wells-Riley equation using $<math><mrow is="true"><mo is="true">〈</mo><mi is="true">C</mi><mo is="true">〉</mo></mrow></math>$ as the airborne pollutant level (instead of the aerosol quantum concentration), as shown in [Eq. (5)](https://www.sciencedirect.com/science/article/pii/S2210670723001440#eqn0005) ([Rudnick & Milton, 2003](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bib0038)). If the quantum generation rate (_r<sub>q</sub>_, h<sup>−1</sup>), breathing rate (_r<sub>b</sub>_, m<sup>3</sup>/h), number of infected people (_N<sub>I</sub>_), and number of people (_N<sub>p</sub>_) remain constant, the average quantum concentration ($<math><mover accent="true" is="true"><mi is="true">Q</mi><mo is="true">¯</mo></mover></math>$) can be equal to the quantum concentration in the exhaled breath of infected people (_r<sub>q</sub>_/_r<sub>b</sub>_) multiplied by average volume fraction of air exhaled by infected people ($<math><mrow is="true"><mover accent="true" is="true"><mi is="true">f</mi><mo is="true">¯</mo></mover><msub is="true"><mi is="true">N</mi><mi is="true">I</mi></msub><mo linebreak="goodbreak" is="true">/</mo><msub is="true"><mi is="true">N</mi><mi is="true">p</mi></msub></mrow></math>$), i.e., $<math><mrow is="true"><mover accent="true" is="true"><mi is="true">Q</mi><mo is="true">¯</mo></mover><mo linebreak="goodbreak" is="true">=</mo><mover accent="true" is="true"><mi is="true">f</mi><mo is="true">¯</mo></mover><msub is="true"><mi is="true">N</mi><mi is="true">I</mi></msub><msub is="true"><mi is="true">r</mi><mi is="true">q</mi></msub><mo linebreak="goodbreak" is="true">/</mo><msub is="true"><mi is="true">N</mi><mi is="true">P</mi></msub><msub is="true"><mi is="true">r</mi><mi is="true">b</mi></msub><mspace width="0.33em" is="true"></mspace><mrow is="true"><mo is="true">[</mo><mrow is="true"><mover accent="true" is="true"><mi is="true">f</mi><mo is="true">¯</mo></mover><mo linebreak="badbreak" is="true">=</mo><mrow is="true"><mo is="true">(</mo><mrow is="true"><mrow is="true"><mo is="true">〈</mo><mi is="true">C</mi><mo is="true">〉</mo></mrow><mo linebreak="badbreak" is="true">−</mo><msub is="true"><mi is="true">C</mi><mrow is="true"><mi is="true">i</mi><mi is="true">n</mi></mrow></msub></mrow><mo is="true">)</mo></mrow><mo linebreak="badbreak" is="true">/</mo><msub is="true"><mi is="true">C</mi><mi is="true">e</mi></msub></mrow><mo is="true">]</mo></mrow></mrow></math>$.(5)$<math><mrow is="true"><msub is="true"><mi is="true">R</mi><mrow is="true"><mi is="true">i</mi><mi is="true">n</mi><mi is="true">f</mi></mrow></msub><mo linebreak="goodbreak" is="true">=</mo><mn is="true">1</mn><mo linebreak="goodbreak" is="true">−</mo><mi is="true">e</mi><mi is="true">x</mi><mi is="true">p</mi><mrow is="true"><mo stretchy="true" is="true">(</mo><mrow is="true"><mo is="true">−</mo><msub is="true"><mi is="true">r</mi><mi is="true">b</mi></msub><mi is="true">t</mi><mover accent="true" is="true"><mi is="true">Q</mi><mo is="true">¯</mo></mover></mrow><mo stretchy="true" is="true">)</mo></mrow><mo linebreak="goodbreak" is="true">=</mo><mn is="true">1</mn><mo linebreak="goodbreak" is="true">−</mo><mi is="true">e</mi><mi is="true">x</mi><mi is="true">p</mi><mrow is="true"><mo stretchy="true" is="true">(</mo><mrow is="true"><mo is="true">−</mo><mfrac is="true"><mrow is="true"><mover accent="true" is="true"><mi is="true">f</mi><mo is="true">¯</mo></mover><msub is="true"><mi is="true">N</mi><mi is="true">I</mi></msub><msub is="true"><mi is="true">r</mi><mi is="true">q</mi></msub><mi is="true">t</mi></mrow><msub is="true"><mi is="true">N</mi><mi is="true">P</mi></msub></mfrac></mrow><mo stretchy="true" is="true">)</mo></mrow></mrow></math>$where, _C<sub>in</sub>_ is the inlet CO<sub>2</sub> concentration (ppm); _C<sub>e</sub>_ is the CO<sub>2</sub> concentration (ppm) added to exhaled breath, which is defined as 38,000 ppm at low levels of oxygen consumption by people ([Rudnick & Milton, 2003](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bib0038)); and _t_ is the exposure time (h), which is defined as 1 h. Estimated _r<sub>q</sub>_ is set to 48 h<sup>−1</sup> ([Dai & Zhao, 2020](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bib0006)), and _N<sub>I</sub>_ is defined as 1, that is, any person can be the infector. The prevention effect is assessed based on the changing rate of CO<sub>2</sub> concentration before and after the decision (through setting the optimal _V<sub>in</sub>_ after the decision). Epidemic prevention level is assessed based on infection risk: “_R<sub>inf</sub>_ ≤ 1%” for “Excellent”, “1% 〈 _R<sub>inf</sub>_  ≤ 1.5%” for “Good”, and “_R<sub>inf</sub>_ 〉 1.5%” for “Poor” ([Dai & Zhao, 2020](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bib0006)).

Cooling load _L<sub>c</sub>_ (kW) is calculated by [Eq. (6)](https://www.sciencedirect.com/science/article/pii/S2210670723001440#eqn0006), including the load by introducing the outdoor air (_L<sub>oa</sub>_, kW), the heat transfer load through the envelope (_L<sub>e</sub>_, kW), and the sensible heat dissipation load emitted by people and equipment (_L<sub>s</sub>_, kW). Latent heat load is not considered here.(6)$<math><mrow is="true"><msub is="true"><mi is="true">L</mi><mi is="true">c</mi></msub><mo linebreak="goodbreak" is="true">=</mo><msub is="true"><mi is="true">L</mi><mrow is="true"><mi is="true">o</mi><mi is="true">a</mi></mrow></msub><mo linebreak="goodbreak" is="true">+</mo><msub is="true"><mi is="true">L</mi><mi is="true">e</mi></msub><mo linebreak="goodbreak" is="true">+</mo><msub is="true"><mi is="true">L</mi><mi is="true">s</mi></msub><mo linebreak="goodbreak" is="true">=</mo><mrow is="true"><mo stretchy="true" is="true">[</mo><mrow is="true"><mtext is="true">OAV</mtext><mrow is="true"><mo stretchy="true" is="true">(</mo><mrow is="true"><msub is="true"><mi is="true">h</mi><mrow is="true"><mi is="true">o</mi><mi is="true">u</mi><mi is="true">t</mi></mrow></msub><mo linebreak="badbreak" is="true">−</mo><msub is="true"><mi is="true">h</mi><mrow is="true"><mi is="true">i</mi><mi is="true">n</mi></mrow></msub></mrow><mo stretchy="true" is="true">)</mo></mrow><mi is="true">ρ</mi><mo linebreak="badbreak" is="true">/</mo><mn is="true">3600</mn></mrow><mo stretchy="true" is="true">]</mo></mrow><mo linebreak="goodbreak" is="true">+</mo><mrow is="true"><mo stretchy="true" is="true">[</mo><mrow is="true"><msub is="true"><mi is="true">K</mi><mi is="true">h</mi></msub><msub is="true"><mi is="true">A</mi><mi is="true">e</mi></msub><mrow is="true"><mo stretchy="true" is="true">(</mo><mrow is="true"><msub is="true"><mi is="true">T</mi><mrow is="true"><mi is="true">o</mi><mi is="true">u</mi><mi is="true">t</mi></mrow></msub><mo linebreak="badbreak" is="true">−</mo><msub is="true"><mi is="true">T</mi><mrow is="true"><mi is="true">i</mi><mi is="true">n</mi></mrow></msub></mrow><mo stretchy="true" is="true">)</mo></mrow><mo linebreak="badbreak" is="true">/</mo><mn is="true">1000</mn></mrow><mo stretchy="true" is="true">]</mo></mrow><mo linebreak="goodbreak" is="true">+</mo><msub is="true"><mi is="true">L</mi><mi is="true">s</mi></msub></mrow></math>$where, _h<sub>out</sub>_ is the air enthalpy outside the room (kJ/kg); _h<sub>in</sub>_ is the air enthalpy inside the room (kJ/kg); _K<sub>h</sub>_ is the external surface heat transfer coefficient of envelope (W/m<sup>2</sup>⋅K); _A<sub>e</sub>_ is the area of envelope (m<sup>2</sup>); _T<sub>out</sub>_ is the outdoor air temperature (°C); and _T<sub>in</sub>_ (°C) is the air temperature inside the room. _T<sub>out</sub>_ is set to be 34.8 °C in Nanjing ([Liu et al., 2019](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bib0022)). Ventilation energy consumption (_EC_, kW) is calculated dividing _L<sub>c</sub>_ with the coefficient of performance (COP). Furthermore, energy efficiency is calculated on account of the ventilation energy consumption before and after the decision.

The decision of optimal _V<sub>in</sub>_ is to balance IAQ, epidemic prevention, and energy consumption. In this study, analytic hierarchy process (AHP) method is used to determine the weightings of pollutant concentration, infection risk, and ventilation energy consumption, and the comprehensive evaluation index _E<sub>cp</sub>_ is established.(7)$<math><mrow is="true"><msub is="true"><mi is="true">E</mi><mrow is="true"><mi is="true">c</mi><mi is="true">p</mi></mrow></msub><mo linebreak="goodbreak" is="true">=</mo><msub is="true"><mi is="true">w</mi><mi is="true">c</mi></msub><msup is="true"><mrow is="true"><mrow is="true"><mo stretchy="true" is="true">〈</mo><mi is="true">C</mi><mo stretchy="true" is="true">〉</mo></mrow></mrow><mo is="true">*</mo></msup><mo linebreak="goodbreak" is="true">+</mo><msub is="true"><mi is="true">w</mi><mrow is="true"><mi is="true">i</mi><mi is="true">n</mi><mi is="true">f</mi></mrow></msub><msubsup is="true"><mi is="true">R</mi><mrow is="true"><mi is="true">i</mi><mi is="true">n</mi><mi is="true">f</mi></mrow><mo is="true">*</mo></msubsup><mo linebreak="goodbreak" is="true">+</mo><msub is="true"><mi is="true">w</mi><mrow is="true"><mi is="true">e</mi><mi is="true">c</mi></mrow></msub><mi is="true">E</mi><msup is="true"><mrow is="true"><mi is="true">C</mi></mrow><mo is="true">*</mo></msup></mrow></math>$where, _w<sub>c</sub>, w<sub>inf</sub>_, and _w<sub>ec</sub>_ are the weightings of dimensionless sub-index $<math><msup is="true"><mrow is="true"><mo is="true">〈</mo><mi is="true">C</mi><mo is="true">〉</mo></mrow><mo is="true">*</mo></msup></math>$, $<math><msubsup is="true"><mi is="true">R</mi><mrow is="true"><mi is="true">i</mi><mi is="true">n</mi><mi is="true">f</mi></mrow><mo is="true">*</mo></msubsup></math>$, and $<math><mrow is="true"><mi is="true">E</mi><msup is="true"><mrow is="true"><mi is="true">C</mi></mrow><mo is="true">*</mo></msup></mrow></math>$. Based on CO<sub>2</sub> concentration field and corresponding _V<sub>in</sub>_ and _N<sub>p</sub>_ conditions from the training database in the rapid prediction model, the samples _E<sub>cp</sub>_ are obtained, and a database of _E<sub>cp</sub>_ is constructed. During the fast decision, the real-time _V<sub>in</sub>_ from ventilation system, CO<sub>2</sub> concentration data from rapid prediction, and _N<sub>p</sub>_ from computer vision detection are used to calculate the real-time _E<sub>cp</sub>_. Then, the differences between the real-time _E<sub>cp</sub>_ and the samples are calculated to obtain the sample _E<sub>cp-sm</sub>_ with a smallest difference and the corresponding _N<sub>p</sub>_. Finally, with the same condition of _N<sub>p</sub>_, all samples _E<sub>cp</sub>_ are traversed to judge whether _E<sub>cp-sm</sub>_ is the minimum value: if _E<sub>cp-sm</sub>_ is minimum, the real-time _V<sub>in</sub>_ is the optimal one; if _E<sub>cp-sm</sub>_ is not minimum, _V<sub>in</sub>_ corresponding to the smallest _E<sub>cp</sub>_ is the optimal one. Optimal _V<sub>in</sub>_ by fast decision is further used for the control of ventilation system, ref., [Section 2.5](https://www.sciencedirect.com/science/article/pii/S2210670723001440#sec0015).

### 2.4. Air purification system

#### 2.4.1. Layout of air purification device

[Table 2](https://www.sciencedirect.com/science/article/pii/S2210670723001440#tbl0002) provides the parameters of a carbon fiber negative ion generator, which are also used in the simulation. The negative ion generators are placed at the height of 0.5 m, and the maximum number of 10 is considered. [Fig. 5](https://www.sciencedirect.com/science/article/pii/S2210670723001440#fig0005) shows the layout of negative ion generators in the hall. The distributions of negative oxygen ions and indoor particles are further compared under different numbers of negative ion generators (_N<sub>g</sub>_ = 2, 4, 6, 8, and 10).

Table 2. Parameters of a negative ion generator.

| Items | Parameters |
| --- | --- |
| Method of negative oxygen ion generation | Carbon fiber brush (negative voltage at −5 kV) |
| Supply air rate of ion nozzle | 0.1 m/s (measured by Testo-405 thermal anemometer) |
| Negative oxygen ion generation rate | 50 million #/cm<sup>3</sup> (measured by AICZX21 ion counter) |
| Detectable released ozone concentration | ≤ 4 ppb ([Han et al., 2008](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bib0017)) |

![Fig 5](https://ars.els-cdn.com/content/image/1-s2.0-S2210670723001440-gr5.jpg)

1.  [Download : Download high-res image (448KB)](https://ars.els-cdn.com/content/image/1-s2.0-S2210670723001440-gr5_lrg.jpg "Download high-res image (448KB)")
2.  [Download : Download full-size image](https://ars.els-cdn.com/content/image/1-s2.0-S2210670723001440-gr5.jpg "Download full-size image")

Fig. 5. Layout of negative ion generators with different numbers of 2, 4, 6, 8, and 10. (For interpretation of the references to color in this figure legend, the reader is referred to the web version of this article.)

#### 2.4.2. Fast decision for air purification system

Fast decision for air purification system is applied to improve the removal efficiency of particles. The optimal number of negative ion generators is determined by comparing the removal efficiencies under different layout strategies. Generally, two negative ion generators are turned on, and then the removal efficiency is calculated. If the removal efficiency is below 70% (the threshold assumed in this work) under two negative ion generators, the number should be increased, and the removal efficiency is further reevaluated. Removal efficiency _η_ (%) is calculated by the particle concentrations when negative ion generators are turned on and off.(8)$<math><mrow is="true"><mi is="true">η</mi><mo linebreak="goodbreak" is="true">=</mo><mrow is="true"><mo stretchy="true" is="true">[</mo><mrow is="true"><mn is="true">1</mn><mo linebreak="badbreak" is="true">−</mo><mfrac is="true"><mrow is="true"><msub is="true"><mi is="true">C</mi><mi is="true">p</mi></msub><mspace width="0.33em" is="true"></mspace><mrow is="true"><mo stretchy="true" is="true">(</mo><mrow is="true"><mi is="true">o</mi><mi is="true">n</mi></mrow><mo stretchy="true" is="true">)</mo></mrow></mrow><mrow is="true"><msub is="true"><mi is="true">C</mi><mi is="true">p</mi></msub><mspace width="0.33em" is="true"></mspace><mrow is="true"><mo stretchy="true" is="true">(</mo><mrow is="true"><mi is="true">o</mi><mi is="true">f</mi><mi is="true">f</mi></mrow><mo stretchy="true" is="true">)</mo></mrow></mrow></mfrac></mrow><mo stretchy="true" is="true">]</mo></mrow><mo linebreak="goodbreak" is="true">×</mo><mspace width="0.33em" is="true"></mspace><mn is="true">100</mn><mo is="true">%</mo></mrow></math>$where, _C<sub>p</sub>_ (_on_) is the particle concentration (#/m<sup>3</sup>) when negative ion generators are turned on; and _C<sub>p</sub>_ (_off_) is the particle concentration (#/m<sup>3</sup>) when negative ion generators are turned off. The optimal number of negative ion generators by fast decision is further used to control air purification system.

### 2.5. Operation, maintenance and control system

By combining limited monitoring modules, rapid prediction models and fast decision methods, the intelligent operation, maintenance, and control system for indoor environment is developed based on five subsystems, as shown in **Appendix C**. Intelligent ventilation system corresponds to subsystems 1, 2, 3 and 4, and air purification system corresponds to subsystem 5.

The visualization platform of intelligent operation, maintenance, and control system is built based on these subsystems (detailed in [Section 3.6](https://www.sciencedirect.com/science/article/pii/S2210670723001440#sec0024)). Regarding the control units, the optimal _V<sub>in</sub>_ for intelligent ventilation system is regulated by the dampers, which are connected to the controller (PCF8591 driver module) to remotely receive the control commands on _V<sub>in</sub>_. Negative ion generators (the number) are controlled by remote power switch. Negative ion generators are turned on when the monitored PM<sub>2.5</sub> concentration exceeds 50 μg/m<sup>3</sup> and the front hall is unoccupied (during 8:00–17:00), to reduce the potential harmful effect of negative ion generator. Then, optimal number of negative ion generators is evaluated by the fast decision for air purification system ([Section 2.4.2](https://www.sciencedirect.com/science/article/pii/S2210670723001440#sec0014)).

[Table 3](https://www.sciencedirect.com/science/article/pii/S2210670723001440#tbl0003) shows an overview of cases in this work. 18 cases are used for the FCM cluster analysis and training of the rapid prediction model. One case is utilized for the validation of rapid prediction model. 15 cases are applied to analyze the effect of negative ion generator on the air purification. The cases for grid independence analysis are not shown here.

Table 3. Overview of cases in this study.

| Empty Cell | Supply air velocity (_V<sub>in</sub>_, m/s) | Number of people (_N<sub>p</sub>_) | Number of negative ion generator (_N<sub>g</sub>_) |
| --- | --- | --- | --- |
| FCM cluster analysis & training of rapid prediction model (18 cases) | 0.7, 1.5, 2.3 | 20, 40, 60, 80, 100, 120 | / |
| Verification of rapid prediction model (1 case) | 0.7 | 50 | / |
| Evaluation of air purification performance (15 cases) | 0.7, 1.5, 2.3 | / | 2, 4, 6, 8, 10 |

## 3\. Results and discussion

First, deployment strategies (number and location) of limited sensors were determined and analyzed. Then, simulations’ prediction was compared with the experiment, to validate the simulation model. Rapid prediction models were validated by the simulation. Optimal deployment of sensors was determined. The air purification effects based on negative oxygen ions and particles were analyzed with different numbers of negative ion generators. Optimal _V<sub>in</sub>_ and number of negative ion generators were evaluated, respectively, based on the CO<sub>2</sub> concentration, infection risk, and ventilation energy consumption, and the removal efficiencies before and after the decision. The visualization platform was established.

### 3.1. Deployment strategy of limited monitoring sensors

In this section, the cluster results by the FCM algorithm were analyzed in **Appendix D**. For different numbers of clusters (sensors), the coordinates of cluster centroids with _V<sub>in</sub>_ = 0.7 m/s were selected as the sensor locations. [Fig. 6](https://www.sciencedirect.com/science/article/pii/S2210670723001440#fig0006) displays the deployment strategies for different numbers of sensors. The optimal deployment strategy of sensors was determined comparing prediction errors (with monitoring data from different number of sensors as inputs), as shown in [Section 3.3](https://www.sciencedirect.com/science/article/pii/S2210670723001440#sec0019).

![Fig 6](https://ars.els-cdn.com/content/image/1-s2.0-S2210670723001440-gr6.jpg)

1.  [Download : Download high-res image (393KB)](https://ars.els-cdn.com/content/image/1-s2.0-S2210670723001440-gr6_lrg.jpg "Download high-res image (393KB)")
2.  [Download : Download full-size image](https://ars.els-cdn.com/content/image/1-s2.0-S2210670723001440-gr6.jpg "Download full-size image")

Fig. 6. Deployment strategies for different numbers of monitoring sensors (_M<sub>s</sub>_ is the monitoring sensor, and _s_ is the serial number of the sensor). (For interpretation of the references to color in this figure legend, the reader is referred to the web version of this article.)

### 3.2. Model validation

The simulation model's prediction was verified by the experiment, and the grid independence analysis was carried out among three different computational grids, namely coarse (1404,586 cells), medium (5233,965 cells), and fine (9490,272 cells), as shown in [Fig. 7](https://www.sciencedirect.com/science/article/pii/S2210670723001440#fig0007). At the validation line (_X_ = 8.3 m, _Y_ = 11.9 m, and _Z_ = 0–15 m), the velocity profiles between medium and fine grids agreed well, showing that the medium grids were adequately resolved in the simulation cases.

![Fig 7](https://ars.els-cdn.com/content/image/1-s2.0-S2210670723001440-gr7.jpg)

1.  [Download : Download high-res image (306KB)](https://ars.els-cdn.com/content/image/1-s2.0-S2210670723001440-gr7_lrg.jpg "Download high-res image (306KB)")
2.  [Download : Download full-size image](https://ars.els-cdn.com/content/image/1-s2.0-S2210670723001440-gr7.jpg "Download full-size image")

Fig. 7. Results of (a) grid independence analysis, and validation between experiment and simulation of (b) velocity and (c) CO<sub>2</sub> concentration. (For interpretation of the references to color in this figure legend, the reader is referred to the web version of this article.)

[Fig. 7](https://www.sciencedirect.com/science/article/pii/S2210670723001440#fig0007)(b) and (c) compare the air velocity and CO<sub>2</sub> concentration between the experimental data and simulation results considering the influence of three _V<sub>in</sub>_. The experiment and simulation showed consistent results on the average velocity. Some differences were noted in average CO<sub>2</sub> concentration as _V<sub>in</sub>_ increased, and the largest difference was 6.8%. In the experiment, people may move in the front hall, which can potentially influence the validation accuracy.

### 3.3. Verification of rapid prediction models using simulation model

#### 3.3.1. Verification of low-dimensional linear model (LLM)

The recomposition performance of uniform and non-uniform LMs was compared based on sub-layer divisions of 6 × 6 × 6, 9 × 9 × 9, 12 × 12 × 12, and 15 × 15 × 15. [Fig. 8](https://www.sciencedirect.com/science/article/pii/S2210670723001440#fig0008) illustrates the velocity fields and compares the CO<sub>2</sub> concentration fields from CFD simulation, uniform LM, and non-uniform LM under the scenario of _V<sub>in</sub>_ = 0.7 m/s and _N<sub>p</sub>_ = 20. The distribution trends of velocity and concentration were similar. With the increased sub-layer number, recomposition performance of LM was improved, which could better reflect the distribution characteristics of CO<sub>2</sub> concentration field. In this study, the recomposition performance of non-uniform LM was superior to that of uniform LM, especially when the sub-layer divisions were 12 × 12 × 12 and 15 × 15 × 15.

![Fig 8](https://ars.els-cdn.com/content/image/1-s2.0-S2210670723001440-gr8.jpg)

1.  [Download : Download high-res image (1006KB)](https://ars.els-cdn.com/content/image/1-s2.0-S2210670723001440-gr8_lrg.jpg "Download high-res image (1006KB)")
2.  [Download : Download full-size image](https://ars.els-cdn.com/content/image/1-s2.0-S2210670723001440-gr8.jpg "Download full-size image")

Fig. 8. Velocity field and comparison of CO<sub>2</sub> concentration fields among CFD simulation, uniform LM, and non-uniform LM under the scenario of _V<sub>in</sub>_ = 0.7 m/s and _N<sub>p</sub>_ = 20 (Note: 6 × 6 × 6, 9 × 9 × 9, 12 × 12 × 12, and 15 × 15 × 15 represent the initial equal divisions of computational domain by 6, 9, 12, and 15 sub-layers in X, Y, and Z directions; 36, 86, 316, and 348 represent the number of zones after self-adaption of initial sub-layer divisions, which is detailed in [Section 2.3.2](https://www.sciencedirect.com/science/article/pii/S2210670723001440#sec0010)). (For interpretation of the references to color in this figure legend, the reader is referred to the web version of this article.)

The discretization errors for uniform and non-uniform LMs were calculated, as shown in [Fig. 9](https://www.sciencedirect.com/science/article/pii/S2210670723001440#fig0009). The discretization error clearly decreased when the number of sub-layers increased. The error index of non-uniform LM outperformed uniform one, indicating its superiority in recomposition accuracy. Moreover, discretization error was analyzed when the sub-layer division was 11 × 11 × 11, which was about 1%−3% higher than 15%. When sub-layer was divided as 12 × 12 × 12, the error of non-uniform LM was below 15%, which can effectively recompose the database.

![Fig 9](https://ars.els-cdn.com/content/image/1-s2.0-S2210670723001440-gr9.jpg)

1.  [Download : Download high-res image (165KB)](https://ars.els-cdn.com/content/image/1-s2.0-S2210670723001440-gr9_lrg.jpg "Download high-res image (165KB)")
2.  [Download : Download full-size image](https://ars.els-cdn.com/content/image/1-s2.0-S2210670723001440-gr9.jpg "Download full-size image")

Fig. 9. Low-dimensional discretization error (%) for uniform and non-uniform LMs under different sub-layers. (For interpretation of the references to color in this figure legend, the reader is referred to the web version of this article.)

[Fig. 10](https://www.sciencedirect.com/science/article/pii/S2210670723001440#fig0010) compares the CO<sub>2</sub> concentration fields between non-uniform LLM and CFD simulation under the scenarios of _V<sub>in</sub>_ = 0.7 m/s and _N<sub>p</sub>_ = 40, 60, 80, 100, and 120. The CO<sub>2</sub> concentration had a substantial increasing trend when _N<sub>p</sub>_ increased. The distribution characteristics of CO<sub>2</sub> concentration fields were similar under different scenarios of _N<sub>p</sub>_, because of the same _V<sub>in</sub>_ resulting in similar velocity fields. For different _N<sub>p</sub>_, non-uniform LLM greatly performed in recomposing the CO<sub>2</sub> concentration database when sub-layers were divided as 12 × 12 × 12. In [Table 4](https://www.sciencedirect.com/science/article/pii/S2210670723001440#tbl0004), the discretization errors of non-uniform LLM were calculated less than 15% compared with the simulation. Moreover, non-uniform LLM can reduce the construction cost of database by over 90% compared with the simulation (from hundreds of megabytes to hundreds of kilobytes), and improve the efficiency of rapid prediction.

![Fig 10](https://ars.els-cdn.com/content/image/1-s2.0-S2210670723001440-gr10.jpg)

1.  [Download : Download high-res image (892KB)](https://ars.els-cdn.com/content/image/1-s2.0-S2210670723001440-gr10_lrg.jpg "Download high-res image (892KB)")
2.  [Download : Download full-size image](https://ars.els-cdn.com/content/image/1-s2.0-S2210670723001440-gr10.jpg "Download full-size image")

Fig. 10. Comparison of CO<sub>2</sub> concentration fields between CFD simulation and non-uniform LLM under the scenarios of _V<sub>in</sub>_ = 0.7 m/s and _N<sub>p</sub>_ = 40, 60, 80, 100, and 120 (Note: 12 × 12 × 12 represent the initial equal divisions of computational domain by 12 sub-layers in X, Y, and Z directions; 316 represent the number of zones after self-adaption of initial sub-layer divisions, which is detailed in [Section 2.3.2](https://www.sciencedirect.com/science/article/pii/S2210670723001440#sec0010)). (For interpretation of the references to color in this figure legend, the reader is referred to the web version of this article.)

Table 4. Results of discretization error for non-uniform LLM.

| _N<sub>p</sub>_ | _V<sub>in</sub>_ (m/s) | _ε_ (%) |
| --- | --- | --- |
| 40 | 0.7 | 13.2 |
| 60 | 0.7 | 14.4 |
| 80 | 0.7 | 13.7 |
| 100 | 0.7 | 14.7 |
| 120 | 0.7 | 14.7 |

#### 3.3.2. Verification of artificial neural network (ANN) based on limited monitoring data

[Fig. 11](https://www.sciencedirect.com/science/article/pii/S2210670723001440#fig0011) compares the CO<sub>2</sub> concentration fields from ANN (with monitoring data from 3, 4, 5, and 6 sensors as inputs) and non-uniform CFD-based LLM under the scenario of _V<sub>in</sub>_ = 0.7 m/s and _N<sub>p</sub>_ = 50. [Fig. 11](https://www.sciencedirect.com/science/article/pii/S2210670723001440#fig0011)(a) illustrates the serial number of non-uniform low-dimensional zones divided by 12 × 12 × 12 sub-layers. A superscript asterisk marked the monitoring zone where the sensors were located. The increased number of sensors improved the prediction performance. When the number of sensors was 3 or 4, the predicted CO<sub>2</sub> concentration fields from ANN were higher or lower than those obtained from CFD-based LLM. When the number of sensors was above 5, the rapid prediction results almost agreed with CFD results, indicating that five sensors could greatly enhance the rapid prediction efficiency.

![Fig 11](https://ars.els-cdn.com/content/image/1-s2.0-S2210670723001440-gr11.jpg)

1.  [Download : Download high-res image (2MB)](https://ars.els-cdn.com/content/image/1-s2.0-S2210670723001440-gr11_lrg.jpg "Download high-res image (2MB)")
2.  [Download : Download full-size image](https://ars.els-cdn.com/content/image/1-s2.0-S2210670723001440-gr11.jpg "Download full-size image")

Fig. 11. (a) Serial number of non-uniform low-dimensional zones, and (b-e) comparisons of CO<sub>2</sub> concentration fields from ANN model (with monitoring data from 3, 4, 5, and 6 sensors as inputs) and CFD-based LLM under the scenario of _V<sub>in</sub>_ = 0.7 m/s and _N<sub>p</sub>_ = 50. (For interpretation of the references to color in this figure legend, the reader is referred to the web version of this article.)

[Fig. 12](https://www.sciencedirect.com/science/article/pii/S2210670723001440#fig0012) shows the prediction errors of ANN (compared with CFD-based LLM) when the number of sensors was 3, 4, 5, and 6, respectively. The threshold of 10% error was selected to judge whether the prediction performance was acceptable. When the number of sensors was 3, the average prediction error was between 15% and 20%, and the minimum error was larger than 10%. When the number of sensors was 4, the average prediction error was less than 10%, yet the maximum error was about 13%, making the ANN potentially ineffective in predicting the local CO<sub>2</sub> concentration field. As the number of sensors was more than five, the maximum prediction error was constantly below 10%, showing an acceptable prediction accuracy. Thus, the optimal number of sensors was determined as 5.

![Fig 12](https://ars.els-cdn.com/content/image/1-s2.0-S2210670723001440-gr12.jpg)

1.  [Download : Download high-res image (184KB)](https://ars.els-cdn.com/content/image/1-s2.0-S2210670723001440-gr12_lrg.jpg "Download high-res image (184KB)")
2.  [Download : Download full-size image](https://ars.els-cdn.com/content/image/1-s2.0-S2210670723001440-gr12.jpg "Download full-size image")

Fig. 12. Absolute prediction errors of ANN model compared with CFD-based LLM with different number of sensors (monitoring zones). (For interpretation of the references to color in this figure legend, the reader is referred to the web version of this article.)

### 3.4. Air purification effect under different numbers of negative ion generators

[Fig. 13](https://www.sciencedirect.com/science/article/pii/S2210670723001440#fig0013) shows the distribution of negative oxygen ions at the plane of _Z_ = 1.7 m with different numbers of negative ion generators and _V<sub>in</sub>_ (0.7, 1.5, and 2.3 m/s). The increased _V<sub>in</sub>_ can lead to the decreased negative oxygen ion concentration. While, increased number of negative ion generators can overcome the disadvantage caused by increased _V<sub>in</sub>_, and remarkedly enhanced the accessibility of negative ions, especially when the number of negative ion generators was above 6.

![Fig 13](https://ars.els-cdn.com/content/image/1-s2.0-S2210670723001440-gr13.jpg)

1.  [Download : Download high-res image (662KB)](https://ars.els-cdn.com/content/image/1-s2.0-S2210670723001440-gr13_lrg.jpg "Download high-res image (662KB)")
2.  [Download : Download full-size image](https://ars.els-cdn.com/content/image/1-s2.0-S2210670723001440-gr13.jpg "Download full-size image")

Fig. 13. Distribution of negative oxygen ions at the plane of _Z_ = 1.7 m under different numbers of negative ion generators and different values of _V<sub>in</sub>_ (0.7, 1.5, and 2.3 m/s). (For interpretation of the references to color in this figure legend, the reader is referred to the web version of this article.)

[Fig. 14](https://www.sciencedirect.com/science/article/pii/S2210670723001440#fig0014) indicates the distributions of relative particle concentration at the plane of _Z_ = 1.7 m for different numbers of negative ion generators and _V<sub>in</sub>_. The distribution of particles was dependent on the spatial concentration of negative oxygen ions. When the number was 2, particles were almost not removed. When the number was increased to 4 and 6, the average concentration at the breathing plane was reduced by around 19%. Furthermore, the concentration was largely reduced by 34% compared with the inlet concentration. When the number was more than 6, the air purification effect was largely improved, and the average concentration was reduced by about 61% compared with the inlet concentration. The increased _V<sub>in</sub>_ could result in increased particle concentrations, since particles were released from the inlets.

![Fig 14](https://ars.els-cdn.com/content/image/1-s2.0-S2210670723001440-gr14.jpg)

1.  [Download : Download high-res image (581KB)](https://ars.els-cdn.com/content/image/1-s2.0-S2210670723001440-gr14_lrg.jpg "Download high-res image (581KB)")
2.  [Download : Download full-size image](https://ars.els-cdn.com/content/image/1-s2.0-S2210670723001440-gr14.jpg "Download full-size image")

Fig. 14. Distribution of relative particle concentration (C<sub>p</sub>/C<sub>0</sub>) at the plane of _Z_ = 1.7 m under different numbers of negative ion generators and different values of _V<sub>in</sub>_ (0.7, 1.5, and 2.3 m/s). (For interpretation of the references to color in this figure legend, the reader is referred to the web version of this article.)

### 3.5. Fast decision analysis for intelligent ventilation and air purification systems

Ventilation performance before (basic scenario) and after the decision was compared over time (from 8:00 to 17:00 with the interval of 30 min) to display the fast decision and control effects. _N<sub>p</sub>_ (obtained by people detection system) before and after the decision (control) were almost similar over time in [Fig. 15](https://www.sciencedirect.com/science/article/pii/S2210670723001440#fig0015)(a), to ensure the reliability of comparing the control effects.

![Fig 15](https://ars.els-cdn.com/content/image/1-s2.0-S2210670723001440-gr15.jpg)

1.  [Download : Download high-res image (327KB)](https://ars.els-cdn.com/content/image/1-s2.0-S2210670723001440-gr15_lrg.jpg "Download high-res image (327KB)")
2.  [Download : Download full-size image](https://ars.els-cdn.com/content/image/1-s2.0-S2210670723001440-gr15.jpg "Download full-size image")

Fig. 15. (a) _N<sub>p</sub>_ and (b) _V<sub>in</sub>_ before the control and after the control (optimal _V<sub>in</sub>_ after the control) changing over time (from 8:00 to 17:00). (For interpretation of the references to color in this figure legend, the reader is referred to the web version of this article.)

[Fig. 15](https://www.sciencedirect.com/science/article/pii/S2210670723001440#fig0015)(b) shows the variations of _V<sub>in</sub>_ before and after the decision (control). The _V<sub>in</sub>_ before the control was set as 0.7 m/s during 8:00–10:00 and 15:00–17:00, and 2.3 m/s during 10:30–14:30. The optimal _V<sub>in</sub>_ after the control was rapidly decided by the comprehensive evaluation index _E<sub>cp</sub>_ in [Eq. (7)](https://www.sciencedirect.com/science/article/pii/S2210670723001440#eqn0007). The weightings for sub-indicators of pollutant concentration, infection risk and energy consumption were calculated by analytic hierarchy process (AHP) method ([Saaty, 1980](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bib0039)), which is detailed in **Appendix E**. The weightings were calculated as 0.31, 0.63, and 0.06 for pollutant concentration, infection risk, and ventilation energy consumption, respectively, and optimal _V<sub>in</sub>_ was obtained by traversing the database of _E<sub>cp</sub>_. Substantial differences were observed in the optimal _V<sub>in</sub>_ for different times. When _N<sub>p</sub>_ changed unpredictably, the fast decision of intelligent ventilation system can rapidly respond to the optimal _V<sub>in</sub>_, based on the online monitoring data of CO<sub>2</sub> concentration and velocity.

[Fig. 16](https://www.sciencedirect.com/science/article/pii/S2210670723001440#fig0016) illustrates the online monitoring data of CO<sub>2</sub> concentration and velocity changing over time before (basic scenario) and after the control based on optimal layouts of sensors. When _V<sub>in</sub>_ was 0.7 m/s before the control, the monitored CO<sub>2</sub> concentration increased to above 1000 ppm, indicating the deterioration of IAQ. As _V<sub>in</sub>_ increased from 0.7 to 2.3 m/s, indoor air velocity increased about two times and CO<sub>2</sub> concentration decreased below 1000 ppm, indicating the improvement of IAQ. After the control, CO<sub>2</sub> concentration was consistently below 1000 ppm, and air velocity was below 0.3 m/s. Compared with the basic scenario before the control, fluctuations of CO<sub>2</sub> concentration and velocity were smaller after the control. The average CO<sub>2</sub> concentrations in the breathing zone before and after the control were rapidly predicted by inputting the monitored CO<sub>2</sub> concentration and velocity.

![Fig 16](https://ars.els-cdn.com/content/image/1-s2.0-S2210670723001440-gr16.jpg)

1.  [Download : Download high-res image (945KB)](https://ars.els-cdn.com/content/image/1-s2.0-S2210670723001440-gr16_lrg.jpg "Download high-res image (945KB)")
2.  [Download : Download full-size image](https://ars.els-cdn.com/content/image/1-s2.0-S2210670723001440-gr16.jpg "Download full-size image")

Fig. 16. Online monitored CO<sub>2</sub> concentration and velocity (a) before the control and (b) after the control changing over time (from 8:00 to 17:00) based on optimal layout strategy of sensors. (For interpretation of the references to color in this figure legend, the reader is referred to the web version of this article.)

[Fig. 17](https://www.sciencedirect.com/science/article/pii/S2210670723001440#fig0017) displays the control effect of average CO<sub>2</sub> concentration before (basic scenario) and after the control, based on the variations of average CO<sub>2</sub> concentration in the breathing region (obtained by non-uniform LLM-based ANN). The increased rate (%) of average CO<sub>2</sub> concentration changing over time was presented as well. During 8:00–10:00 and 15:00–17:00 with _V<sub>in</sub>_ of 0.7 m/s, the average CO<sub>2</sub> concentration before the control was mostly above 1000 ppm, and increased by 45% when compared with that after the control. During 10:30–14:30 with _V<sub>in</sub>_ of 2.3 m/s, the average concentration before the control was almost less than 1000 ppm and reduced by around 50% compared with that after the control. Generally, the average CO<sub>2</sub> concentration after the control based on optimal _V<sub>in</sub>_ was constantly less than 1000 ppm to satisfy the requirement of good IAQ, which proved the effectiveness of fast decision of optimal _V<sub>in</sub>_ in intelligent ventilation.

![Fig 17](https://ars.els-cdn.com/content/image/1-s2.0-S2210670723001440-gr17.jpg)

1.  [Download : Download high-res image (321KB)](https://ars.els-cdn.com/content/image/1-s2.0-S2210670723001440-gr17_lrg.jpg "Download high-res image (321KB)")
2.  [Download : Download full-size image](https://ars.els-cdn.com/content/image/1-s2.0-S2210670723001440-gr17.jpg "Download full-size image")

Fig. 17. Average CO<sub>2</sub> concentration $<math><mrow is="true"><mo is="true">〈</mo><mi is="true">C</mi><mo is="true">〉</mo></mrow></math>$ in the breathing region (obtained by rapid prediction model) and increased rate of $<math><mrow is="true"><mo is="true">〈</mo><mi is="true">C</mi><mo is="true">〉</mo></mrow></math>$ changing over time before and after the control. (For interpretation of the references to color in this figure legend, the reader is referred to the web version of this article.)

Next, control effects of infection risk and energy consumption for intelligent ventilation system were compared. [Fig. 18](https://www.sciencedirect.com/science/article/pii/S2210670723001440#fig0018)(a) shows the variation of infection risk over time before (basic scenario) and after the control. For the scenario of before the control, when _V<sub>in</sub>_ was lower, infection risk was higher than 1.5% limitation, indicating the poor epidemic prevention level. After the control, infection risk was constantly less than 1.5%, satisfying the requirement of good epidemic prevention. The variation of ventilation energy consumption over time before and after the control is presented in [Fig. 18](https://www.sciencedirect.com/science/article/pii/S2210670723001440#fig0018)(b). The ventilation energy consumption after the control largely decreased by 27.4% compared with that before the control. With the increased _V<sub>in</sub>_ after the control (compared with 0.7 m/s before the control), the energy consumption after the control was increased by about 25% when compared with that before the control, at the time moment of 9:30. Increased energy consumption was mainly utilized to reduce the pollutant concentration and control the infection risk, shown in [Figs. 17](https://www.sciencedirect.com/science/article/pii/S2210670723001440#fig0017) and [18](https://www.sciencedirect.com/science/article/pii/S2210670723001440#fig0018)(a). Therefore, the fast decision of intelligent ventilation could contribute to the favorable control effects.

![Fig 18](https://ars.els-cdn.com/content/image/1-s2.0-S2210670723001440-gr18.jpg)

1.  [Download : Download high-res image (401KB)](https://ars.els-cdn.com/content/image/1-s2.0-S2210670723001440-gr18_lrg.jpg "Download high-res image (401KB)")
2.  [Download : Download full-size image](https://ars.els-cdn.com/content/image/1-s2.0-S2210670723001440-gr18.jpg "Download full-size image")

Fig. 18. (a) Infection risk and (b) ventilation energy consumption changing over time before and after the control. (For interpretation of the references to color in this figure legend, the reader is referred to the web version of this article.)

According to [Fig. 14](https://www.sciencedirect.com/science/article/pii/S2210670723001440#fig0014), removal efficiencies for different numbers of negative ion generators and _V<sub>in</sub>_ were calculated, as shown in [Fig. 19](https://www.sciencedirect.com/science/article/pii/S2210670723001440#fig0019). When negative ion generators were turned off, the particle concentration was assumed 11,000 #/cm<sup>3</sup>. Removal efficiency increased gradually with an increased number of negative ion generator. When the number was 2, the removal efficiency was the lowest by approaching to about 20%. When the number was ≥ 8, the maximum removal efficiency was 81.2%. Removal efficiency was reduced when _V<sub>in</sub>_ decreased. The limitation of 70% removal efficiency was assumed to determine the optimal number of negative ion generators. Thus, the optimal number was 6 when _V<sub>in</sub>_ was 0.7 m/s, and the optimal number was 8 when _V<sub>in</sub>_ was 1.5 and 2.3 m/s.

![Fig 19](https://ars.els-cdn.com/content/image/1-s2.0-S2210670723001440-gr19.jpg)

1.  [Download : Download high-res image (607KB)](https://ars.els-cdn.com/content/image/1-s2.0-S2210670723001440-gr19_lrg.jpg "Download high-res image (607KB)")
2.  [Download : Download full-size image](https://ars.els-cdn.com/content/image/1-s2.0-S2210670723001440-gr19.jpg "Download full-size image")

Fig. 19. Removal efficiencies for different numbers of negative ion generators (2, 4, 6, 8, and 10) and different _V<sub>in</sub>_ (0.7, 1.5, and 2.3 m/s). (For interpretation of the references to color in this figure legend, the reader is referred to the web version of this article.)

### 3.6. Platform of intelligent operation, maintenance, and control for indoor environment

The visualization platform for intelligent operation, maintenance, and control system is shown in [Fig. 20](https://www.sciencedirect.com/science/article/pii/S2210670723001440#fig0020). The left column exhibited the control effect of intelligent ventilation, including energy efficiency, rapid prediction of CO<sub>2</sub> concentration, and control effect of CO<sub>2</sub> concentration. Regarding the column of energy efficiency, real-time ventilation load, ventilation rate, opening degree of damper, and energy saving rate were displayed. Non-uniform low-dimensional linear model (LLM)-based artificial neural network (ANN) can obtain the rapidly predicted CO<sub>2</sub> concentrations with limited monitoring data as inputs. For the control effect of CO<sub>2</sub> concentration, average CO<sub>2</sub> concentration in the breathing region (based on rapid prediction model), _N<sub>p</sub>_ (based on occupant detection), and IAQ (based on average CO<sub>2</sub> concentration in the breathing area) were presented, respectively.

![Fig 20](https://ars.els-cdn.com/content/image/1-s2.0-S2210670723001440-gr20.jpg)

1.  [Download : Download high-res image (1MB)](https://ars.els-cdn.com/content/image/1-s2.0-S2210670723001440-gr20_lrg.jpg "Download high-res image (1MB)")
2.  [Download : Download full-size image](https://ars.els-cdn.com/content/image/1-s2.0-S2210670723001440-gr20.jpg "Download full-size image")

Fig. 20. Visualization platform of intelligent operation, maintenance and control for indoor environment in the front hall of the International Convention and Exhibition Center. (For interpretation of the references to color in this figure legend, the reader is referred to the web version of this article.)

The right column displayed the epidemic prevention and removal effect, consisting of effect of epidemic prevention, distribution of removal efficiency, and total removal effect and total IAQ. The effect of epidemic prevention included prevention effect, infection risk, and epidemic prevention level. The distribution removal efficiency was displayed based on the distribution of negative oxygen ions and particles at the breathing plane of _Z_ = 1.7 m. The real-time monitored PM<sub>2.5</sub>, negative oxygen ion (the average value at the breathing plane), TVOC, temperature, and RH were shown in the column of total removal effect and total IAQ.

The middle column illustrated the fast decision results for removal efficiency, energy efficiency, epidemic prevention level, and IAQ, to emphasize the coupling performance of intelligent ventilation and air purification. Although the increased ventilation rate can lead to the decreased negative oxygen ion concentration, the increased number of air purification device also improved the pollutant removal effect. Generally, negative ion generators potentially reduce the pollutant concentration and infection risk combined with intelligent ventilation. In-depth coupling of intelligent ventilation and purification systems can be further considered in future work, to enhance the performance of intelligent operation, maintenance, and control of building environment. Moreover, the energy consumption of 10 negative ion generators was approximately 50 W, which was negligible compared with the ventilation energy consumption in summer conditions.

## 4\. Conclusions

With respect to the non-uniform distributions of public building environment, this study develops an intelligent operation, maintenance, and control system in the front hall of the International Convention and Exhibition Center, by coupling intelligent ventilation and air purification systems. The pollutant concentration fields are rapidly predicted using non-uniform low-dimensional linear model (LLM)-based artificial neural network (ANN) model, based on optimal deployment strategy of limited sensors. Optimal inlet velocity (_V<sub>in</sub>_) and number of negative ion generators are evaluated, respectively, through fast decision of ventilation and air purification performance. Ventilation and air purification systems are controlled in real time based on the evaluation results. Finally, a visualization platform of intelligent operation, maintenance, and control system is established to effectively display the effects for online monitoring, rapid prediction, fast decision, and dynamic control. The main conclusions are as follows.

-   (1)
    
    Taking limited monitoring data from the optimal deployment of sensors (number of 5) as inputs, the prediction error of CO<sub>2</sub> concentration field by ANN model based on non-uniform LLM (sub-layer number of 12) was less than 10% compared with CFD simulation.
    
-   (2)
    
    Fast decision of intelligent ventilation can rapidly respond to the optimal _V<sub>in</sub>_, which largely improved the ventilation performance by reducing CO<sub>2</sub> concentration below 1000 ppm, infection risk below 1.5%, and ventilation energy consumption by 27.4%.
    
-   (3)
    
    Using the air purification system, the average particle concentration was largely reduced by 61% compared with the inlet concentration. By fast decision for air purification system, the maximum removal efficiency was 81.2% when the number of negative ion generators was 10.
    
-   (4)
    
    By establishing a visualization platform, the intelligent operation, maintenance, and control system showed the control effects of intelligent ventilation, epidemic prevention, pollutant removal, and energy savings efficiently, through coupling the intelligent ventilation and air purification systems.
    

The limitations and future work are discussed. (i) This study develops the intelligent operation, maintenance, and control system (platform) of indoor environment in a front hall. However, the full-scale system for public building is not developed, and also the effects of intelligent ventilation control system are not verified by the experimental data, which should be the focus of future work. (ii) The coupling analysis between intelligent ventilation and air purification systems is considered, while the future work should also integrate the ventilation rate, infection risk, energy efficiency, removal effect, and IAQ deeply. Particularly, the mutual positive and negative influences between ventilation and air purification systems should be considered. (iii) There are still uncertainties about evaluation criteria of IAQ for intelligent ventilation control system, especially when applied in different countries or regions due to the potential limitations of different standards or recommendations. It can be favorable to establish the reliable values of criteria to improve the performance of intelligent ventilation control, which will be studied in future work. (iv) The real-time monitoring of occupant location and behavior should be also considered in the intelligent operation, maintenance, and control system to improve the prediction and control performance of dynamic building environment. (v) The intelligent operation, maintenance, and control system (platform) should be applied to more types of public buildings (e.g., hospitals, offices, and schools), and the diversification of indoor environmental parameters (including different pollutant types) should be also considered, in order to improve the integrated performance of operation, maintenance, and control.

## Declaration of Competing Interest

The authors declare that they have no known competing financial interests or personal relationships that could have appeared to influence the work reported in this paper.

## Acknowledgement

The authors would like to acknowledge the supports from the National Natural Science Foundation of China (Grant No. [52178069](https://www.sciencedirect.com/science/article/pii/S2210670723001440#gs00001)), and the National Natural Science Funds for Distinguished Young Scholar (Grant No. 52225005), and the Funds for International Cooperation and Exchange of the National Natural Science Foundation of China (Grant No. [52211530036](https://www.sciencedirect.com/science/article/pii/S2210670723001440#gs00002)).

## Data availability

-   Data will be made available on request.
    

## References

1.  [Ai and Melikov, 2018](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bbib0001)
    
    Airborne spread of expiratory droplet nuclei between the occupants of indoor environments: A review
    
    Indoor Air, 28 (2018), pp. 500-524
    
2.  [Alghamdi, Tang, Kanjanabootra and Alterman, 2023](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bbib0002)
    
    S. Alghamdi, W. Tang, S. Kanjanabootra, D. Alterman
    
    Field investigations on thermal comfort in university classrooms in New South Wales, Australia
    
    Energy Reports, 9 (2023), pp. 63-71
    
3.  [Cao, Ding and Ren, 2020](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bbib0003)
    
    S.J. Cao, J.W. Ding, C. Ren
    
    Sensor deployment strategy using cluster analysis of Fuzzy C-Means Algorithm: Towards online control of indoor environment's safety and health
    
    Sustainable Cities and Society, 59 (2020), Article 102190
    
4.  [Chen et al., 2020](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bbib0004)
    
    X.J. Chen, K. Qu, J. Calautit, A. Ekambaram, W. Lu, C. Fox, …, S. Riffat
    
    Multi-criteria assessment approach for a residential building retrofit in Norway
    
    Energy Buildings, 215 (2020), Article 109668
    
5.  [Chow, 2002](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bbib0005)
    
    W.K. Chow
    
    Ventilation of enclosed train compartments in Hong Kong
    
    Applied Energy, 71 (2002), pp. 161-170
    
6.  [Dai and Zhao, 2020](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bbib0006)
    
    H. Dai, B. Zhao
    
    Association of the infection probability of COVID-19 with ventilation rates in confined spaces
    
    Building Simulation, 13 (2020), pp. 1321-1327
    
7.  [Ding, Yu and Cao, 2020](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bbib0007)
    
    J.W. Ding, C.W. Yu, S.J. Cao
    
    HVAC systems for environmental control to minimize the COVID-19 infection
    
    Indoor + Built Environment : The Journal of the International Society of the Built Environment, 29 (2020), pp. 1195-1201
    
8.  [D'Orazio and D'Alessandro, 2020](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bbib0008)
    
    A. D'Orazio, D. D'Alessandro
    
    Air bio-contamination control in hospital environment by UV-C rays and HEPA filters in HVAC systems
    
    Annali di Igiene: Medicina Preventiva e di Comunita, 32 (2020), pp. 449-461
    
9.  [Feng et al., 2020](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bbib0009)
    
    Q.L. Feng, H. Cai, Y.B. Yang, J.H. Xu, M.R. Jiang, F. Li, X.T. Li, C.C. Yan
    
    An experimental and numerical study on a multi-robot source localization method independent of airflow information in dynamic indoor environments
    
    Sustainable Cities and Society, 53 (2020), Article 101897
    
10.  [Feng, Cao and Haghighat, 2021](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bbib0010)
    
    Z.B. Feng, S.J. Cao, F. Haghighat
    
    Removal of SARS-CoV-2 using UV+Filter in built environment
    
    Sustainable Cities and Society, 74 (2021), Article 103226
    
11.  [Feng, Wei, Li and Yu, 2021](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bbib0011)
    
    Z.B. Feng, F.Z. Wei, H. Li, C.W. Yu
    
    Evaluation of indoor disinfection technologies for airborne disease control in hospital
    
    Indoor + Built Environment : The Journal of the International Society of the Built Environment, 30 (2021), pp. 727-731
    
12.  [Fischer et al., 2020](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bbib0012)
    
    R.J. Fischer, D.H. Morris, N. van Doremalen, S. Sarchette, M.J. Matson, T. Bushmaker, C.K. Yinda, S.N. Seifert, A. Gamble, B.N. Williamson, S.D. Judson, E. de Wit, J.O. Lloyd-Smith, V.J. Munster
    
    Effectiveness of N95 Respirator Decontamination and Reuse against SARS-CoV-2 Virus
    
    Emerging Infectious Diseases, 26 (2020), pp. 2253-2255
    
13.  [GB50736-2012 2012](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bbib0014)
    
    GB50736-2012
    
    Design code for heating ventilation and air conditioning of civil buildings
    
    Ministry of Housing and Urban-Rural Development, China (2012)
    
    [Google Scholar](https://scholar.google.com/scholar_lookup?title=Design%20code%20for%20heating%20ventilation%20and%20air%20conditioning%20of%20civil%20buildings&publication_year=2012&author=GB50736-2012)
    
14.  [GB/T18883-2022, 2022](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bbib0013)
    
    GB/T18883-2022
    
    Indoor air quality standard
    
    Ministry of Ecology and Environment of the People’s Republic of China (2022)
    
    [Google Scholar](https://scholar.google.com/scholar_lookup?title=Indoor%20air%20quality%20standard&publication_year=2022&author=GB%2FT18883-2022)
    
15.  [Grabarczyk, 2001](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bbib0015)
    
    Z. Grabarczyk
    
    Effectiveness of indoor air cleaning with corona ionizers
    
    Journal of Electrostatics, 51 (2001), pp. 278-283
    
16.  [Gupta, Anand and Gupta, 2017](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bbib0016)
    
    P. Gupta, S. Anand, H. Gupta
    
    Developing a roadmap to overcome barriers to energy efficiency in buildings using best worst method
    
    Sustainable Cities and Society, 31 (2017), pp. 244-259
    
17.  [Han, Kim, Kim and Sioutas, 2008](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bbib0017)
    
    B. Han, H.J. Kim, Y.J. Kim, C. Sioutas
    
    Unipolar charging of fine and ultra-fine particles using carbon fiber ionizers
    
    Aerosol Science and Technology, 42 (2008), pp. 793-800
    
18.  [Huang et al., 2022](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bbib0018)
    
    H. Huang, H.L. Wang, Y.J. Hu, C.J. Li, X.L. Wang
    
    Optimal plan for energy conservation and CO2 emissions reduction of public buildings considering users' behavior: Case of China
    
    Energy, 261 (2022), Article 125037
    
19.  [Kumar et al., 2016](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bbib0019)
    
    P. Kumar, A.N. Skouloudis, M. Bell, M. Viana, M.C. Carotta, G. Biskos, L. Morawska
    
    Real-time sensors for indoor air monitoring and challenges ahead in deploying them to urban buildings
    
    Science of the Total Environment, 560 (2016), pp. 150-159
    
20.  [Li and Cai, 2022](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bbib0020)
    
    B.X. Li, W.J. Cai
    
    A novel CO2-based demand-controlled ventilation strategy to limit the spread of COVID-19 in the indoor environment
    
    Building Environment, 219 (2022), Article 109232
    
21.  [Li et al., 2019](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bbib0021)
    
    S.S. Li, S.Y. Zhang, W.X. Pan, Z.W. Long, T. Yu
    
    Experimental and theoretical study of the collection efficiency of the two-stage electrostatic precipitator
    
    Powder Technology, 356 (2019), pp. 1-10
    
22.  [Liu et al., 2019](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bbib0022)
    
    M.Z. Liu, C.G. Zhu, H. Zhang, W.D. Zheng, S.J. You, P.E. Campana, J.Y. Yan
    
    The environment and energy consumption of a subway tunnel by the influence of piston wind
    
    Applied Energy, 246 (2019), pp. 11-23
    
23.  [Lu, Pang, Fu and Neill, 2022](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bbib0023)
    
    X. Lu, Z.H. Pang, Y.Y. Fu, Z.O. Neill
    
    Advances in research and applications of CO2-based demand-controlled ventilation in commercial buildings: A critical review of control strategies and performance evaluation
    
    Building and Environment, 223 (2022), Article 109455
    
24.  [Lu, Pang, Fu and O'Neill, 2022](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bbib0024)
    
    X. Lu, Z.H. Pang, Y.Y. Fu, Z. O'Neill
    
    The nexus of the indoor CO2 concentration and ventilation demands underlying CO2-based demand-controlled ventilation in commercial buildings: A critical review
    
    Building and Environment, 218 (2022), Article 109116
    
25.  [Ma, Liu and Shang, 2021](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bbib0025)
    
    G.F. Ma, T.Y. Liu, S.S. Shang
    
    Improving the climate adaptability of building green retrofitting in different regions: A weight correction system for Chinese national standard
    
    Sustainable Cities and Society, 69 (2021), Article 102843
    
26.  [Merema, Delwati, Sourbron and Breesch, 2018](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bbib0026)
    
    B. Merema, M. Delwati, M. Sourbron, H. Breesch
    
    Demand controlled ventilation (DCV) in school and office buildings: Lessons learnt from case studies
    
    Energy and Buildings, 172 (2018), pp. 349-360
    
27.  [Nakpan et al., 2019](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bbib0027)
    
    W. Nakpan, M. Yermakov, R. Lndugula, T. Reponen, S.A. Grinshpun
    
    Inactivation of bacterial and fungal spores by UV irradiation and gaseous iodine treatment applied to air handling filters
    
    Science of the Total Environment, 671 (2019), pp. 59-65
    
28.  [Palmisani et al., 2021](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bbib0028)
    
    J. Palmisani, A. Di Gilio, M. Viana, G. de Gennaro, A. Ferro
    
    Indoor air quality evaluation in oncology units at two European hospitals: Low-cost sensors for TVOCs, PM2.5 and CO2 real-time monitoring
    
    Building and Environment, 205 (2021), Article 108237
    
29.  [Paterson, Sharpe, Taylor and Morrissey, 2021](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bbib0029)
    
    C.A. Paterson, R.A. Sharpe, T. Taylor, K. Morrissey
    
    Indoor PM2.5, VOCs and asthma outcomes: A systematic review in adults and their home environments
    
    Environmental Research, 202 (2021), Article 111631
    
30.  [Pushpawela, Jayaratne, Nguy and Morawska, 2017](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bbib0030)
    
    B. Pushpawela, R. Jayaratne, M. Nguy, L. Morawska
    
    Efficiency of ionizers in removing airborne particles in indoor environments
    
    Journal of Electrostatics, 90 (2017), pp. 79-84
    
31.  [Ren and Cao, 2019a](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bbib0031)
    
    C. Ren, S.J. Cao
    
    Development and application of linear ventilation and temperature models for indoor environmental prediction and HVAC systems control
    
    Sustainable Cities and Society, 51 (2019), Article 101673
    
32.  [Ren and Cao, 2020](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bbib0032)
    
    C. Ren, S.J. Cao
    
    Implementation and visualization of artificial intelligent ventilation control system using fast prediction models and limited monitoring data
    
    Sustainable Cities and Society, 52 (2020), Article 101860
    
33.  [Ren, Cao and Haghighat, 2022](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bbib0033)
    
    C. Ren, S.J. Cao, F. Haghighat
    
    A practical approach for preventing dispersion of infection disease in naturally ventilated room
    
    Journal of Building Engineering, 48 (2022), Article 103921
    
34.  [Ren et al., 2022](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bbib0034)
    
    C. Ren, F. Haghighat, Z.B. Feng, P. Kumar, S.J. Cao
    
    Impact of ionizers on prevention of airborne infection in classroom
    
35.  [Ren, Zhu and Cao, 2022](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bbib0035)
    
    C. Ren, H.C. Zhu, S.J. Cao
    
    Ventilation strategies for mitigation of infection disease transmission in an indoor environment: A case study in office
    
    Buildings, 12 (2022), p. 180
    
36.  [Ren and Cao, 2019b](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bbib0036)
    
    J. Ren, S.J. Cao
    
    Incorporating online monitoring data into fast prediction models towards the development of artificial intelligent ventilation systems
    
    Sustainable Cities and Society, 47 (2019), Article 101498
    
37.  [Rohde, Larsen, Jensen and Larsen, 2020](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bbib0037)
    
    L. Rohde, T.S. Larsen, R.L. Jensen, O.K. Larsen
    
    Framing holistic indoor environment: Definitions of comfort, health and well-being
    
    Indoor + Built Environment : The Journal of the International Society of the Built Environment, 29 (2020), pp. 1118-1136
    
38.  [Rudnick and Milton, 2003](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bbib0038)
    
    S.N. Rudnick, D.K. Milton
    
    Risk of indoor airborne infection transmission estimated from carbon dioxide concentration
    
    Indoor Air, 13 (2003), pp. 237-245
    
    [Google Scholar](https://scholar.google.com/scholar_lookup?title=Risk%20of%20indoor%20airborne%20infection%20transmission%20estimated%20from%20carbon%20dioxide%20concentration&publication_year=2003&author=S.N.%20Rudnick&author=D.K.%20Milton)
    
39.  [Saaty, 1980](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bbib0039)
    
    T.L. Saaty
    
    The analytic hierarchy process: Planning, priority setting, resource allocation
    
    McGraw-Hill (1980)
    
    [Google Scholar](https://scholar.google.com/scholar_lookup?title=The%20analytic%20hierarchy%20process%3A%20Planning%2C%20priority%20setting%2C%20resource%20allocation&publication_year=1980&author=T.L.%20Saaty)
    
40.  [Shiue, Hu and Tu, 2011](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bbib0040)
    
    A. Shiue, S.C. Hu, M.L. Tu
    
    Particles removal by negative ionic air purifier in cleanroom
    
    Aerosol and Air Quality Research, 11 (2011), pp. 179-186
    
41.  [Su, Miao, Wang and Wang, 2022](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bbib0041)
    
    Y. Su, Z.Y. Miao, L.W. Wang, L.Y. Wang
    
    Energy consumption and indoor environment evaluation of large irregular commercial green building in Dalian, China
    
    Energy and Buildings, 276 (2022), Article 112506
    
42.  [Wang et al., 2021](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bbib0042)
    
    J.Q. Wang, J.J. Huang, Z.B. Feng, S.J. Cao, F. Haghighat
    
    Occupant-density-detection based energy efficient ventilation system: Prevention of infection transmission
    
    Energy and Buildings, 240 (2021), Article 110883
    
43.  [Xue, Wang, Liu and Dong, 2020](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bbib0043)
    
    Q.W. Xue, Z.J. Wang, J. Liu, J.K. Dong
    
    Indoor PM2.5 concentrations during winter in a severe cold region of China: A comparison of passive and conventional residential buildings
    
    Building and Environment, 180 (2020), Article 106857
    
44.  [Yu et al., 2017](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bbib0044)
    
    S. Yu, Y.L. Ma, G.J. Zhang, W. Wang, G.H. Feng
    
    Numerical simulation study on location optimization of indoor air purifiers in bedroom
    
    Procedia Engineering, 205 (2017), pp. 849-855
    
    [Google Scholar](https://scholar.google.com/scholar_lookup?title=Numerical%20simulation%20study%20on%20location%20optimization%20of%20indoor%20air%20purifiers%20in%20bedroom&publication_year=2017&author=S.%20Yu&author=Y.L.%20Ma&author=G.J.%20Zhang&author=W.%20Wang&author=G.H.%20Feng)
    
45.  [Zhang and Zhang, 2007](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bbib0045)
    
    Q. Zhang, G.Q. Zhang
    
    Study on TVOCs concentration distribution and evaluation of inhaled air quality under a re-circulated ventilation system
    
    Building and Environment, 42 (2007), pp. 1110-1118
    
46.  [Zhang and You, 2017](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bbib0046)
    
    T.H. Zhang, X.Y. You
    
    The use of genetic algorithm and self-updating artificial neural network for the inverse design of cabin environment
    
    Indoor + Built Environment : The Journal of the International Society of the Built Environment, 26 (2017), pp. 347-354
    
47.  [Zhang et al., 2020](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bbib0047)
    
    X.L. Zhang, X.F. Li, Z.C. Wang, G.F. Deng, Z.Y. Wang
    
    Exposure level and influential factors of HCHO, BTX and TVOC from the interior redecoration of residences
    
    Building and Environment, 168 (2020), Article 106494
    
48.  [Zhu, Ren and Cao, 2022](https://www.sciencedirect.com/science/article/pii/S2210670723001440#bbib0048)
    
    H.C. Zhu, C. Ren, S.J. Cao
    
    Dynamic sensing and control system using artificial intelligent techniques for non-uniform indoor environment
    
    Building and Environment, 226 (2022), Article 109702
    

## Cited by (0)

[View Abstract](https://www.sciencedirect.com/science/article/abs/pii/S2210670723001440)

© 2023 Elsevier Ltd. All rights reserved.
