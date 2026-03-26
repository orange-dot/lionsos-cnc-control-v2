<!--
     Copyright 2026

     SPDX-License-Identifier: MIT
-->

# CNC Control V2 Backlog Template

Koristi ovaj sablon za svaki novi backlog dokument u `docs/backlog/`.

Poenta sablona je da backlog ostane dovoljno precizan za AI implementaciju i
ljudski review, bez nejasnog "sredi nekako" prostora.

## Kada se koristi

Koristi ga za:

- novi veci backlog pravac
- novu backlog temu koja ima vise work item-a
- backlog koji treba da se sece na vise AI-ready slice-eva

Nemoj ga koristiti za:

- veci arhitektonski manifest ili README
- jednokratnu odluku bez backlog razrade
- jedan mali coding task koji vec ima jasan slice brief

## Required Sections

Svaki backlog dokument mora da ima sledece sekcije:

- `Svrha`
- `Status`
- `Reality Check`
- `Definition Of Done`
- `Canonical Artifacts`
- `Dependencies`
- `Work Items`
- `Out Of Scope`
- `Human-Owned Decisions`

Opcione sekcije:

- `Chosen Defaults`
- `Acceptance Sequence`
- `Risks`
- `Future Slices`

## Canonical Skeleton

```md
# <kratko backlog ime>

## Svrha

- sta backlog pokriva
- gde mu je granica

## Status

Ovaj backlog je trenutno **planned|active|blocked|partial|done**.

## Reality Check

- sta danas postoji
- sta nedostaje
- koje kontradikcije ili rupe postoje

## Definition Of Done

- koji uslovi moraju istovremeno biti tacni
- kako znamo da je backlog stvarno zatvoren

## Canonical Artifacts

- koji fajlovi, header-i, generated artefakti ili docs su kanonski

## Dependencies

- koji drugi backlogovi ili docs moraju vec biti zakljucani

## Work Items

### Bxx-001 <kratko ime>

Zadatak:

- sta treba zakljucati ili isporuciti

Acceptance:

- kako znamo da je ovaj item gotov

Status:

- planned|active|blocked|partial|done

## Out Of Scope

- sta backlog namerno ne pokriva

## Human-Owned Decisions

- sta AI ne sme sam da prelama
- koje odluke ostaju za coveka
```

## Rules For Good Backlogs

- cilj mora biti opisan kao ishod, ne kao mehanicki spisak komandi
- `reality check` mora reci sta je stvarno danasnje stanje, ne samo idealnu metu
- work item-i moraju imati stabilne ID-jeve
- svaki work item mora imati acceptance i status
- backlog mora imati eksplicitan `out of scope`
- ako postoji visoko-impact odluka, ona mora stajati u
  `Human-Owned Decisions`, ne biti sakrivena u tekstu

## Slice Extraction Rules

Kada backlog sazri za implementaciju, iz njega izvuci poseban slice brief.

Svaki slice mora da zakljuca:

- jedan glavni cilj
- mali broj fajlova ili jedan jasan subsystem
- stable boundaries koje ne sme da pomeri
- acceptance koji moze da se proveri

Ako jedan slice prirodno trazi vise nezavisnih acceptance koraka, treba ga
rastaviti na vise slice-eva.
