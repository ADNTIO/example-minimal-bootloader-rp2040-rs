# Bonnes pratiques CI/CD

Ce document décrit le pipeline cible du projet. Son principe essentiel est de
construire une seule fois, de valider exactement ce qui a été construit, puis de
publier ces mêmes fichiers sans les reconstruire.

## Vue d'ensemble

```mermaid
flowchart LR
    SOURCE([Push / Pull request]) --> L

    subgraph CI[CONTINUOUS INTEGRATION]
        direction LR
        L[1. Lint] --> B[2. Build]
        B --> T[3. Tests]
        T --> S[4. SBOM]
        S --> SEC[5. Security check]
        SEC --> H[6. Tests HIL]
        H --> GATE{Pipeline vert ?}
    end

    GATE -->|Non| STOP([Pipeline bloqué])
    GATE -->|Oui| TAG{Tag vX.Y.Z ?}
    TAG -->|Non| VALID([Commit validé])

    subgraph CD[CONTINUOUS DELIVERY]
        direction LR
        R[7. Créer la release] --> D[8. Publier les SBOM<br/>Dependency-Track]
        D --> A[9. Publier les assets]
        A --> DOC[10. Publier la documentation]
    end

    TAG -->|Oui| R

    B -. mêmes binaires .-> H
    B -. mêmes binaires .-> A
    S -. mêmes SBOM .-> D

    classDef trigger fill:#2563eb,color:#fff,stroke:#1d4ed8,stroke-width:2px;
    classDef ci fill:#e0f2fe,color:#0c4a6e,stroke:#0284c7;
    classDef gate fill:#fef3c7,color:#78350f,stroke:#f59e0b,stroke-width:2px;
    classDef cd fill:#dcfce7,color:#14532d,stroke:#16a34a;
    classDef success fill:#16a34a,color:#fff,stroke:#15803d,stroke-width:2px;
    classDef failure fill:#dc2626,color:#fff,stroke:#b91c1c,stroke-width:2px;

    class SOURCE trigger;
    class L,B,T,S,SEC,H ci;
    class GATE,TAG gate;
    class R,D,A,DOC cd;
    class VALID success;
    class STOP failure;
```

## Pipeline de validation

Le pipeline est lancé pour chaque commit poussé et chaque pull request. Chaque
étape ne démarre que si la précédente a réussi.

1. **Lint** : vérifie le formatage et les règles Rust, Python et Markdown. Cette
   étape rapide échoue avant de consommer du temps de compilation.
2. **Build** : compile le bootloader, les firmwares et les outils. Les fichiers
   obtenus sont enregistrés comme artefacts du workflow.
3. **Tests** : exécute les tests unitaires et les tests logiciels sans matériel.
4. **SBOM** : produit les inventaires CycloneDX à partir du commit et des
   dépendances verrouillées utilisés pour le build.
5. **Security check** : analyse les SBOM avec Grype et Trivy. Une vulnérabilité
   dépassant le seuil défini, par exemple `HIGH`, bloque le pipeline.
6. **HIL (Hardware-in-the-Loop)** : télécharge les artefacts du build, les
   installe sur une carte réelle et exécute les tests d'intégration. Ce job doit
   utiliser un runner dédié, sérialisé par carte pour éviter deux tests
   simultanés sur le même matériel.

Un échec arrête la chaîne. Aucune release ni publication ne doit avoir lieu à
partir d'un pipeline incomplet.

## Pipeline de release

Un commit validé n'est pas automatiquement une nouvelle version. La publication
est déclenchée par un tag versionné, par exemple `v1.2.3`, posé sur un commit dont
tous les contrôles précédents ont réussi.

1. **Création de la release** : crée la version et ses notes à partir du tag.
2. **Publication SBOM** : envoie à Dependency-Track les SBOM déjà contrôlés. Le
   nom du projet, la version et le hash du commit doivent identifier précisément
   la release.
3. **Publication des assets** : joint à la release les binaires déjà testés par
   le HIL, ainsi que leurs sommes de contrôle. Aucun rebuild n'est effectué.
4. **Publication de la documentation** : publie la documentation correspondant
   au même tag et ajoute un lien vers la release.

Si une publication échoue, elle doit pouvoir être relancée avec les artefacts du
même pipeline, sans recréer une version ni recompiler les binaires.

## Règles à conserver

- Un seul build et une seule génération de SBOM par commit.
- Les scans, le HIL et la release consomment les artefacts produits en amont.
- Les secrets de Dependency-Track et de publication sont accessibles uniquement
  aux jobs de release, idéalement via un environnement GitHub protégé.
- Les actions et les outils sont épinglés à des versions connues.
- Une seule exécution est autorisée par carte HIL.
- Les artefacts, les SBOM et la documentation portent tous la même version et le
  même commit source.
