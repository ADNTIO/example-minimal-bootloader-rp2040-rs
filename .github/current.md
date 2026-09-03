# Analyse du pipeline actuel et cible

## Problèmes actuels

Le pipeline est réparti entre `ci.yml`, `security-scan.yml` et `sbom.yml`. Cette
séparation ne garantit pas qu’une release et sa publication Dependency-Track
proviennent d’un scan réussi.

| Problème | Conséquence | Modification |
|---|---|---|
| `security-scan.yml` écoute les branches, pas les tags | Un tag de release ne lance ni Grype ni Trivy dans ce workflow | Déclencher un workflow unique sur tous les `push`, tags inclus |
| La release de `ci.yml` ne dépend pas des scans | Une release peut être créée malgré une CVE | Faire dépendre la release d’un job de sécurité réussi |
| `sbom.yml` utilise une autre génération SBOM et seulement Grype | Trivy ne protège pas la publication Dependency-Track | Générer une seule fois puis analyser le même artefact avec les deux outils |
| `sbom.yml` accepte tous les tags, tandis que la release accepte `v*` | Un tag non versionné peut être envoyé à Dependency-Track | Valider strictement `^v[0-9]+\.[0-9]+\.[0-9]+$` |
| Trois workflows s’exécutent indépendamment | Pas de garde commun avant les publications | Remplacer les trois chemins par `pipeline.yml` |
| `check`, `test` et `build` tournent sur tous les pushes | Travail inutile par rapport au besoin exprimé | Les exécuter uniquement pour un tag de release valide |

## Pipeline cible

```mermaid
%% Flowchart — une SBOM par push, un garde de sécurité commun et une release sur tag.
flowchart TD
    push(("Push d’un commit<br>branche ou tag")):::trigger
    sbom["Générer une seule fois la SBOM<br>cdxgen, CycloneDX 1.6"]:::primary

    subgraph security["Sécurité — exécutée à chaque push"]
        direction LR
        grype["Analyser la SBOM<br>avec Grype"]
        trivy["Analyser la SBOM<br>avec Trivy"]
    end

    releaseTag{"Tag strict<br>vX.Y.Z ?"}:::decision

    subgraph releaseChecks["En parallèle de la SBOM — tag uniquement"]
        direction LR
        check["Format + lint"]
        test["Tests unitaires"]
        build["Build des binaires"]
    end

    securityGate{"Au moins une CVE détectée<br>par Grype ou Trivy ?"}:::decision
    releaseGate{"Check, tests et build réussis ?"}:::decision
    publishGate{"Sécurité et validation<br>réussies ?"}:::decision
    blocked(["Pipeline en échec<br>aucune publication"]):::error
    commitDone(["Commit contrôlé"]):::success
    githubRelease["Créer la GitHub Release<br>avec les binaires et les SBOM"]:::success
    dtrack["Publier la même SBOM<br>dans Dependency-Track"]:::success

    push --> sbom
    push --> releaseTag
    sbom --> grype
    sbom --> trivy
    releaseTag -->|non| commitDone
    releaseTag -->|oui| check
    releaseTag -->|oui| test
    releaseTag -->|oui| build
    grype --> securityGate
    trivy --> securityGate
    securityGate -->|oui| blocked
    securityGate -->|non, sans tag| commitDone
    securityGate -->|non, avec tag| publishGate
    check --> releaseGate
    test --> releaseGate
    build --> releaseGate
    releaseGate -->|non| blocked
    releaseGate -->|oui| publishGate
    publishGate --> githubRelease
    publishGate --> dtrack

    classDef trigger fill:#fed7aa,stroke:#c2410c,color:#374151
    classDef success fill:#a7f3d0,stroke:#047857,color:#374151
    classDef decision fill:#fef3c7,stroke:#b45309,color:#374151
    classDef error fill:#fecaca,stroke:#b91c1c,color:#374151
    classDef primary fill:#3b82f6,stroke:#1e3a5f,color:#ffffff
```

## Dépendances de jobs recommandées

```text
push
├── sbom → grype + trivy ───────────┐
└── tag vX.Y.Z → check + test + build ┴── garde finale
                                           ├── GitHub Release
                                           └── Dependency-Track
```

La SBOM et les binaires sont transmis entre jobs avec les artefacts GitHub
Actions. `release` et `dtrack` attendent les deux scans, le lint, les tests et le
build, mais ces deux groupes s’exécutent désormais en parallèle.

## Changements réalisés

1. `workflows/pipeline.yml` contient désormais cette chaîne unique.
2. Le workflow réutilise `make lint`, `make test-unit`, `make all` et
   `scripts/ci/prepare-release-assets.sh` déjà présents.
3. `workflows/ci.yml`, `workflows/security-scan.yml` et `workflows/sbom.yml` ont
   été supprimés pour éviter les exécutions et publications en double.

Le diagramme applique littéralement « une CVE trouvée = échec ». Cela implique
un seuil incluant toutes les sévérités dans Grype et Trivy ; il pourra être
resserré à `HIGH,CRITICAL` si les vulnérabilités faibles ne doivent pas bloquer.
