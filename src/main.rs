use clap::{Args, Parser, Subcommand};
use serde_json::Value;
use std::{
    fs::File,
    io::{BufReader, BufWriter, Write},
    path::PathBuf,
};

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

                for element in doc.elements.iter() {
                    if let Some(element_part) = get_slice_after_last_occurrence(&element.id, '.') {
                        let hier_level = count_char_occurrences(&element.id, '.') * 2;

                        // if the datatype is one of the classes drawn, add a relation instead of a class element
                        // TODO: element is removed from element list if there is one datatype that is among the structure definitions
                        let mut show_element = true;
                        for datatype in element.datatype.iter() {
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
                    "| Code | Element | Datatype | Cardinality | Preferred Code System | Binding Strength |"
                )?;
                writeln!(writer, "| --- | --- | --- | --- | --- | --- |")?;

                let mut levels = Vec::<usize>::new();
                levels.push(0);
                let mut current_level: usize = 0;

                for element in doc.elements.iter() {
                    if let Some(element_part) = get_slice_after_last_occurrence(&element.id, '.') {
                        let hier_level: usize = count_char_occurrences(&element.id, '.');
                        if hier_level > current_level {
                            levels.push(1);
                            current_level += 1;
                        } else if hier_level < current_level {
                            levels.pop();
                            current_level -= 1;
                        } else {
                            levels[current_level] += 1;
                        }
                        let mut code = args.prefix_code.clone();
                        for lv in 1..(current_level + 1) {
                            code.push('.');
                            code.push_str(&levels[lv].to_string());
                        }

                        write!(
                            writer,
                            "| {} | {} | {} | {} |",
                            code,
                            camel_to_spaced_pascal(&element_part.replace("[x]", "")),
                            reduce_datatypes(&element.datatype),
                            format!("{}..{}", element.min, element.max)
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
    }

    Ok(())
}

fn load_json_from_file(path: &String) -> Result<Value, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let value = serde_json::from_reader(reader)?;
    Ok(value)
}

fn get_slice_after_last_occurrence(s: &String, c: char) -> Option<String> {
    if let Some(last_index) = s.rfind(c) {
        Some(s[last_index + c.len_utf8()..].to_string())
    } else {
        None
    }
}

fn count_char_occurrences(s: &String, c: char) -> usize {
    s.chars().filter(|&ch| ch == c).count()
}

fn load_structure_definition_files(
    files: &Vec<String>,
) -> Result<Vec<DocInfo>, Box<dyn std::error::Error>> {
    let mut docs = Vec::<DocInfo>::new();
    for file in files.iter() {
        match load_single_structure_definition_file(file) {
            Ok(doc_info) => {
                docs.push(doc_info);
            }
            Err(e) => {
                println!("Error: {}", e);
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
        if element_id != id {
            let mut datatype = Vec::<String>::new();
            for dt in element["type"].as_array().ok_or("Missing datatype")? {
                if let Some(code) = dt["code"].as_str() {
                    let code = code.to_string();
                    if code.starts_with("http") {
                        match get_slice_after_last_occurrence(&code, '/') {
                            Some(end) => {
                                datatype.push(end);
                            }
                            None => {}
                        };
                    } else {
                        datatype.push(code);
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
                datatype,
                min,
                max: max.to_string(),
                binding,
                binding_strength,
            });
        }
    }
    Ok(DocInfo {
        id: id.to_string(),
        elements,
    })
}

fn camel_to_spaced_pascal(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c.is_uppercase() && !result.is_empty() {
            result.push(' ');
        }
        result.push(c);
    }

    result
        .split_whitespace()
        .map(|word| {
            let mut c = word.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn reduce_datatypes(datatypes: &Vec<String>) -> String {
    let mut result = String::new();
    let mut first = true;
    for d in datatypes.iter() {
        if !first {
            result.push_str(", ");
            first = false;
        };
        result.push_str(d);
    }
    result
}
