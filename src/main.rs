mod utils;

use crate::utils::{count_char_occurrences, get_slice_after_last_occurrence, load_json_from_file};
use clap::{Args, Parser, Subcommand};
use std::{
    fs::File,
    io::{BufWriter, Write},
    path::PathBuf,
};
use utils::{camel_to_spaced_pascal, reduce_datatypes};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    /// Command
    #[command(subcommand)]
    command: Commands,
}

#[derive(Args, Debug)]
struct CommonArgs {
    /// Files to process
    files: Vec<String>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Generate plantUml class diagrams in a single file based on the structure definitions
    PlantUml(PlantUmlArgs),
    /// Generate a plantUml mind map in a separate file for each structure definition
    Mindmap(MindmapArgs),
    /// Generate a markdown table in a separate file for each structure definition
    Table(TableArgs),
}

#[derive(Args, Debug)]
struct PlantUmlArgs {
    #[command(flatten)]
    common: CommonArgs,

    /// Hide data elements
    #[arg(short, long)]
    elements_hide: bool,

    /// Hide cardinality
    #[arg(short, long)]
    cardinality_hide: bool,

    /// Output tile name
    #[arg(short, long, default_value = "output.plantuml")]
    output_file: PathBuf,
}

#[derive(Args, Debug)]
struct MindmapArgs {
    #[command(flatten)]
    common: CommonArgs,

    /// At which hierarchical level to stop using boxes in mind map
    #[arg(short, long, default_value_t = 255)]
    box_level: usize,
}

#[derive(Args, Debug)]
struct TableArgs {
    #[command(flatten)]
    common: CommonArgs,

    /// Prefix used for code generation
    #[arg(short, long, default_value = "A")]
    prefix_code: String,
}

#[derive(Debug)]
struct ElementInfo {
    id: String,
    short: String,
    definition: String,
    datatype: Vec<String>,
    min: String,
    max: String,
    binding: Option<String>,
    binding_strength: Option<String>,
}

#[derive(Debug)]
struct DocInfo {
    id: String,
    elements: Vec<ElementInfo>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::PlantUml(args) => {
            let docs = load_structure_definition_files(&args.common.files)?;
            let output = File::create(args.output_file)?;
            let mut writer = BufWriter::new(output); // Create a buffered writer

            writeln!(
                writer,
                "@startuml\nskinparam linetype polyline\nhide circle\nhide stereotype\nhide methods\n"
            )?;

            for doc in docs.iter() {
                println!("processing: {}", doc.id);
                writeln!(writer, "class **{}** {{", doc.id)?;
                let mut relations = Vec::<(String, String, String, String)>::new();

                // TODO: Assumes elements appear in the right order. Could sort on ElementDefinition.id but that would change the stated order.
                for element in doc.elements.iter() {
                    if let Some(element_part) = get_slice_after_last_occurrence(&element.id, '.') {
                        let hier_level = count_char_occurrences(&element.id, '.') * 2;

                        // if the datatype is one of the classes drawn, add a relation instead of a class element
                        // TODO: element is removed from element list if there is one datatype that is among the structure definitions
                        let mut show_element = true;
                        for datatype in element.datatype.iter() {
                            // TODO: or use a hashmap for faster lookup
                            // TODO: look also for Reference(X or T)
                            if let Some(_) = docs.iter().position(|d| datatype == &d.id) {
                                relations.push((
                                    element_part.clone(),
                                    datatype.clone(),
                                    element.min.clone(),
                                    element.max.clone(),
                                ));
                                show_element = false;
                            }
                        }

                        if show_element && !args.elements_hide {
                            write!(
                                writer,
                                "{:>hier_level$}|_ {} : {}",
                                "",
                                element_part,
                                reduce_datatypes(&element.datatype)
                            )?;
                            if !args.cardinality_hide {
                                write!(writer, " [{}..{}]", element.min, element.max)?;
                            }
                            writeln!(writer)?;
                        }
                    }
                }

                writeln!(writer, "}}")?;

                for rel in relations {
                    writeln!(
                        writer,
                        "\"**{}**\" -- \"{}..{}\" \"**{}**\" : {} >",
                        doc.id,
                        rel.2,
                        rel.3,
                        rel.1,
                        rel.0.replace("[x]", "")
                    )?;
                }

                writeln!(writer)?;
            }

            writeln!(writer, "@enduml")?;
        }
        Commands::Mindmap(args) => {
            let docs = load_structure_definition_files(&args.common.files)?;
            for doc in docs.iter() {
                println!("processing: {}", doc.id);
                let output = File::create(format!("{}.plantuml", doc.id))?;
                let mut writer = BufWriter::new(output); // Create a buffered writer

                writeln!(writer, "@startmindmap")?;

                writeln!(writer, "* {}", doc.id)?;

                for element in doc.elements.iter() {
                    if let Some(element_part) = get_slice_after_last_occurrence(&element.id, '.') {
                        let hier_level: usize = count_char_occurrences(&element.id, '.') + 1;
                        writeln!(
                            writer,
                            "{}{} {}",
                            "*".repeat(hier_level),
                            if hier_level > args.box_level { "_" } else { "" },
                            camel_to_spaced_pascal(&element_part.replace("[x]", ""))
                        )?;
                    }
                }

                writeln!(writer, "@endmindmap")?;
            }
        }
        Commands::Table(args) => {
            let docs = load_structure_definition_files(&args.common.files)?;
            for doc in docs.iter() {
                println!("processing: {}", doc.id);
                let output = File::create(format!("{}.md", doc.id))?;
                let mut writer = BufWriter::new(output); // Create a buffered writer

                writeln!(writer, "# {}", doc.id)?;

                writeln!(
                    writer,
                    "| Code | Element | Short | Definition | Datatype | Cardinality | Preferred Code System | Binding Strength |"
                )?;
                writeln!(writer, "| --- | --- | --- | --- | --- | --- | --- | --- |")?;

                let mut levels = Vec::<usize>::new();
                levels.push(0);
                let mut current_level: usize = 0;

                for element in doc.elements.iter() {
                    let hier_level: usize = count_char_occurrences(&element.id, '.');
                    let element_part: String = if hier_level > 0 {
                        get_slice_after_last_occurrence(&element.id, '.')
                            .ok_or("Wrong element part")?
                    } else {
                        element.id.clone()
                    };
                    match hier_level.cmp(&current_level) {
                        std::cmp::Ordering::Greater => {
                            levels.push(1);
                            current_level += 1;
                        }
                        std::cmp::Ordering::Less => {
                            levels.pop();
                            current_level -= 1;
                        }
                        std::cmp::Ordering::Equal => {
                            levels[current_level] += 1;
                        }
                    }
                    let mut code = args.prefix_code.clone();
                    for level in &levels[1..=current_level] {
                        code.push('.');
                        code.push_str(&level.to_string());
                    }

                    write!(
                        writer,
                        "| {} | {} | {} | {} | {} | {}..{} |",
                        code,
                        camel_to_spaced_pascal(&element_part.replace("[x]", "")),
                        element.short,
                        element.definition,
                        reduce_datatypes(&element.datatype),
                        element.min, element.max
                    )?;
                    if let Some(binding) = &element.binding {
                        write!(writer, " {} |", binding)?;
                    } else {
                        write!(writer, " |")?;
                    }
                    if let Some(binding_strength) = &element.binding_strength {
                        write!(writer, " {} |", binding_strength)?;
                    } else {
                        write!(writer, " |")?;
                    }
                    writeln!(writer)?;
                }
            }
        }
    }

    Ok(())
}

fn load_structure_definition_files(
    files: &[String],
) -> Result<Vec<DocInfo>, Box<dyn std::error::Error>> {
    let mut docs = Vec::<DocInfo>::new();
    for file in files.iter() {
        match load_single_structure_definition_file(file) {
            Ok(doc_info) => {
                docs.push(doc_info);
            }
            Err(e) => {
                println!("Error reading file '{}': {}", file, e);
            }
        }
    }
    Ok(docs)
}

fn load_single_structure_definition_file(
    file: &String,
) -> Result<DocInfo, Box<dyn std::error::Error>> {
    let doc = load_json_from_file(file)?;
    let id = doc["id"].as_str().ok_or("Missing id")?;
    let snapshot = doc["snapshot"]["element"]
        .as_array()
        .ok_or("Missing snapshot")?;
    let mut elements = Vec::<ElementInfo>::new();
    for element in snapshot.iter() {
        let element_id = element["id"].as_str().ok_or("Missing element id")?;
        let short = element["short"]
            .as_str()
            .ok_or("Missing short description")?
            .to_string();
        let definition = element["definition"]
            .as_str()
            .ok_or("Missing definition")?
            .to_string();
        let mut datatype = Vec::<String>::new();
        if let Some(type_array) = element["type"].as_array() {
            for dt in type_array {
                if let Some(code) = dt["code"].as_str() {
                    let code = code.to_string();
                    if code.starts_with("http") {
                        if let Some(end) = get_slice_after_last_occurrence(&code, '/') {
                            datatype.push(end);
                        };
                    } else if code == "Reference" {
                        // TODO: does not distunguish between Reference and direct datatype
                        if let Some(profiles) = dt["targetProfile"].as_array() {
                            for profile_value in profiles {
                                if let Some(profile) = profile_value.as_str() {
                                    let profile = profile.to_string();
                                    if let Some(end) = get_slice_after_last_occurrence(&profile, '/') {
                                        datatype.push(end);
                                    };
                                }
                                
                            }
                        }
                    } else {
                        datatype.push(code);
                    }
                }
            }
        }

        let min = if element["min"].is_string() {
            element["min"]
                .as_str()
                .ok_or(format!("Missing min cardinality: {:?}", element["min"]))?
                .to_string()
        } else {
            element["min"].to_string()
        };

        let max = element["max"].as_str().ok_or("Missing max cardinality")?;
        let binding = element["binding"]["description"]
            .as_str()
            .map(|s| s.to_string());
        let binding_strength = element["binding"]["strength"]
            .as_str()
            .map(|s| s.to_string());

        elements.push(ElementInfo {
            id: element_id.to_string(),
            short,
            definition,
            datatype,
            min,
            max: max.to_string(),
            binding,
            binding_strength,
        });

    }
    Ok(DocInfo {
        id: id.to_string(),
        elements,
    })
}
