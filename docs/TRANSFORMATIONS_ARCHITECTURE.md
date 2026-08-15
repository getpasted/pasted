# Historical Transformations Architecture

Status: superseded before 1.0.

This document previously described Pipelines as a separate product and persistence model. That proposal was replaced when reusable workflows were consolidated as Transforms.

Use [Transformations](TRANSFORMATIONS.md) for the current product and execution contracts. See [Transform storage decision for 1.0](TRANSFORM_STORAGE_DECISION.md) for the migration and compatibility boundary. Remaining `Pipeline` names are internal pre-release adapters or accepted legacy input; canonical storage, stable references, GUI language, and CLI output use Transform.
