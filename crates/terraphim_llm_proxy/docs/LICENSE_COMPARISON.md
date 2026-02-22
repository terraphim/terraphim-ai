# License Comparison and Rationale

**Document Date:** 2026-02-02
**Selected License:** FSL-1.1-MIT (Functional Source License v1.1 with MIT Future License)

---

## Executive Summary

Terraphim LLM Proxy uses the **FSL-1.1-MIT** license, which provides source-available rights with automatic conversion to MIT after two years. This document compares FSL-1.1-MIT against alternatives considered, including 37signals' O'Saasy License and plain MIT.

---

## Licenses Compared

### 1. FSL-1.1-MIT (Selected)

**Origin:** Sentry (Functional Software, Inc.)
**SPDX Identifier:** FSL-1.1-MIT

**Key Features:**
- Source-available with broad permissions
- Restricts "Competing Use" (offering as competing SaaS)
- Automatic conversion to MIT after 2 years
- Comprehensive patent grant and termination clauses
- Explicit trademark protections
- Clear enumeration of permitted purposes

**Permitted Purposes:**
1. Internal use and access
2. Non-commercial education
3. Non-commercial research
4. Professional services for other licensees

**Competing Use Definition:**
Making the Software available to others in a commercial product or service that:
1. Substitutes for the Software
2. Substitutes for any other product/service offered by licensor
3. Offers same or substantially similar functionality

### 2. O'Saasy License (37signals)

**Origin:** 37signals LLC (December 2025)
**Used By:** Fizzy (Kanban tool)

**Full License Text:**
```
Permission is hereby granted, free of charge, to any person obtaining a copy 
of this software and associated documentation files (the "Software"), to deal 
in the Software without restriction, including without limitation the rights 
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell 
copies of the Software, and to permit persons to whom the Software is 
furnished to do so, subject to the following conditions:

1. The above copyright notice and this permission notice shall be included 
   in all copies or substantial portions of the Software.
2. No licensee or downstream recipient may use the Software (including any 
   modified or derivative versions) to directly compete with the original 
   Licensor by offering it to third parties as a hosted, managed, or 
   Software-as-a-Service (SaaS) product or cloud service where the primary 
   value of the service is the functionality of the Software itself.

THE SOFTWARE IS PROVIDED "AS IS"...
```

**Key Features:**
- Essentially MIT + SaaS non-compete clause
- Very short and simple
- No patent provisions
- No trademark provisions
- No automatic license conversion
- Vague "primary value" language

### 3. MIT License

**Origin:** Massachusetts Institute of Technology
**Used By:** 37signals ONCE products (Campfire, Writebook) since August 2025

**Key Features:**
- Fully permissive
- No restrictions on use
- No patent grant (implicit)
- No protection against competing SaaS offerings

---

## Feature Comparison Matrix

| Feature | FSL-1.1-MIT | O'Saasy | MIT |
|---------|-------------|---------|-----|
| Self-hosting allowed | Yes | Yes | Yes |
| Modification allowed | Yes | Yes | Yes |
| Commercial use | Yes (non-competing) | Yes (non-competing SaaS) | Yes |
| Sell copies | During FSL: restricted | Yes | Yes |
| **SaaS competition restriction** | Yes (detailed) | Yes (vague) | No |
| **Patent grant** | Yes | No | No (implicit) |
| **Patent termination clause** | Yes | No | No |
| **Trademark protection** | Yes | No | No |
| **Permitted purposes enumerated** | Yes | No | N/A |
| **Future license conversion** | Yes (2 years -> MIT) | No | N/A |
| **Legal comprehensiveness** | High | Low | Medium |

---

## Legal Robustness Analysis

### FSL-1.1-MIT Strengths

1. **Patent Protection**
   - Explicit patent grant for permitted purposes
   - Automatic termination if licensee makes patent claims
   - Protects both licensor and good-faith licensees

2. **Clear Definitions**
   - "Licensor", "Software", "Permitted Purpose" explicitly defined
   - Three-part test for "Competing Use"
   - No ambiguous "primary value" language

3. **Trademark Clause**
   - Explicitly reserves trademark rights
   - Permits identification of origin only
   - Prevents brand confusion

4. **Future License Grant**
   - Irrevocable MIT conversion after 2 years
   - Provides certainty for long-term adopters
   - Balances protection with eventual openness

### O'Saasy Weaknesses

1. **Vague Restriction Language**
   > "where the primary value of the service is the functionality of the Software itself"
   
   What constitutes "primary value"? 51%? 80%? This invites litigation.

2. **No Patent Provisions**
   - No explicit patent grant
   - No protection against patent trolls
   - Licensee has no patent license

3. **No Trademark Protection**
   - Licensors have no explicit trademark reservation
   - Could lead to brand confusion

4. **No Conversion Path**
   - Remains restrictive indefinitely
   - No certainty for adopters

---

## Industry Context

### 37signals License Evolution

| Date | Product | License |
|------|---------|---------|
| 2024 | Campfire (ONCE) | Proprietary ($299) |
| Aug 2025 | Campfire | MIT (open-sourced) |
| Dec 2025 | Fizzy | O'Saasy |

### Fair Source Movement

FSL-1.1-MIT is part of the [Fair Source](https://fair.io/) movement, which aims to provide:
- User freedom (use, modify, redistribute)
- Developer sustainability (protection from free-riding)
- Eventual full openness (automatic conversion)

Companies using FSL include:
- Sentry (creator of FSL)
- GitButler
- Codecov

### Controversy

Matt Mullenweg [criticized DHH](https://ma.tt/2025/12/dhh-open-source/) for calling Fizzy "open source" when it has competition restrictions. The Fair Source community [invited 37signals](https://openpath.quest/2025/fizzy-should-be-fair-source/) to adopt FSL instead of creating O'Saasy.

---

## Conclusion

**FSL-1.1-MIT is the optimal choice for terraphim-llm-proxy because:**

1. **Legal Robustness**: Comprehensive coverage of patents, trademarks, and permitted uses
2. **Clear Definitions**: No vague "primary value" language that invites disputes
3. **Industry Backing**: Created and maintained by Sentry's legal team
4. **Future Openness**: Automatic MIT conversion provides certainty
5. **Fair Source Alignment**: Part of established movement with clear principles

The O'Saasy license, while simpler, lacks critical protections and uses ambiguous language. Plain MIT provides no protection against competing SaaS offerings.

---

## References

- [FSL Official Site](https://fsl.software/)
- [FSL-1.1-MIT on SPDX](https://spdx.org/licenses/FSL-1.1-MIT.html)
- [Fair Source Licenses](https://fair.io/licenses/)
- [Fizzy O'Saasy License](https://github.com/basecamp/fizzy/blob/main/LICENSE.md)
- [ONCE License (MIT)](https://once.com/license)
- [Matt Mullenweg on DHH & Open Source](https://ma.tt/2025/12/dhh-open-source/)
- [Fizzy Should Be Fair Source](https://openpath.quest/2025/fizzy-should-be-fair-source/)
