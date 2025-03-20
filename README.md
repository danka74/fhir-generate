# FHIR Generate

FHIR Generate is a tool designed to help developers generate visualisation artefacts (currently UML and mindmap diagrams and table) from FHIR StructureDefinitions in json format.

## Table of Contents
- [FHIR Generate](#fhir-generate)
  - [Table of Contents](#table-of-contents)
  - [Building](#building)
  - [Usage](#usage)
  - [Contributing](#contributing)

## Building

FHIR Generate is written in RUST and a working RUST environment is needed to build the tool. To install FHIR Generate, clone the repository and install the necessary dependencies:

```bash
git clone https://github.com/yourusername/fhir-generate.git
cd fhir-generate
cargo build
```

## Usage

To generate artefacts, run the following command:

```bash
fhir-generate <type of artefact> <options> <structuredefinition files>*
```
e.g.
```bash
fhir-generate table *.json

fhir-generate plant-uml -e plant-uml -e StructureDefinition-EHDSMedicationDispense.json StructureDefinition-EHDSMedication.json
```

## Contributing

We welcome contributions! Please follow these steps to contribute:

1. Fork the repository.
2. Create a new branch (`git checkout -b feature-branch`).
3. Make your changes.
4. Commit your changes (`git commit -m 'Add new feature'`).
5. Push to the branch (`git push origin feature-branch`).
6. Open a pull request.

