mod utils;

use crate::utils::{
    count_char_occurrences, generate_code, get_slice_after_first_occurrence, get_slice_after_last_occurrence, load_json_from_file
};
use clap::{Args, Parser, Subcommand};
use easy_tree::Tree;
use fmt_derive::Display;
use std::{
    //    collections::{HashMap, HashSet},
    collections::{HashMap, HashSet},
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
    files: Vec<PathBuf>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Generate plantUml class diagrams in a single file based on the structure definitions
    PlantUml(PlantUmlArgs),
    /// Generate a plantUml mind map in a separate file for each structure definition
    Mindmap(MindmapArgs),
    /// Generate a markdown table in a separate file for each structure definition
    Table(TableArgs),
    /// Generate a markdown table in a single file based on obligations of a structure definition
    Obligations(ObligationsArgs),
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

    /// Hide code
    #[arg(short, long)]
    code_hide: bool,

    /// Prefix used for code generation
    #[arg(short, long, default_value = "A")]
    prefix_code: String,
}

#[derive(Args, Debug)]
struct ObligationsArgs {
    #[command(flatten)]
    common: CommonArgs,

    /// Prefix used for code generation
    #[arg(short, long, default_value = "A")]
    prefix_code: String,

    /// Folder with actor definitions
    #[arg(short, long)]
    actors_folder: Option<PathBuf>,    
}

#[derive(Debug, Clone, Display, Hash, PartialEq, Eq, PartialOrd, Ord)]
struct ElementInfo {
    id: String,
    short: String,
    definition: String,
    datatype: Vec<String>,
    min: String,
    max: String,
    global_min: String,
    global_max: String,
    binding: Option<String>,
    binding_strength: Option<String>,
    obligation: Vec<(String, String, String)>,
    requirements: Option<String>,
}

struct StructureDefTreeInfo {
    id: String,
    element_tree: Tree<ElementInfo>,
}

trait SearchableTree<T> {
    fn find_first<F>(&self, predicate: F) -> Option<usize>
    where
        F: Fn(&T) -> bool;
}

impl SearchableTree<ElementInfo> for Tree<ElementInfo> {
    fn find_first<F>(&self, predicate: F) -> Option<usize>
    where
        F: Fn(&ElementInfo) -> bool,
    {
        for node in self.iter() {
            if predicate(node.1) {
                return Some(node.0);
            }
        }
        None
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Table(args) => {
            // first load all structure definitions into in-memory structs
            let docs = load_structure_definition_files(&args.common.files)?;
            let alpha_index_code = args.prefix_code == "A";
            for (doc_num, doc) in docs.iter().enumerate() {
                let prefix = if alpha_index_code {
                    generate_code(doc_num)
                } else {
                    args.prefix_code.clone()
                };
                println!("processing: {}", doc.id);
                let output = File::create(format!("{}.md", doc.id))?;
                let mut writer = BufWriter::new(output); // Create a buffered writer

                writeln!(writer, "# {}", doc.id)?;

                writeln!(
                    writer,
                    "| Code | Path | Element | Description | Datatype | Cardinality | Global Cardinality | Preferred Code System | Requirements |"
                )?;
                writeln!(
                    writer,
                    "| --- | --- | --- | --- | --- | --- | --- | --- | --- "
                )?;

                let mut levels = Vec::<usize>::new();
                levels.push(0);
                let mut current_level: usize = 0;
                // sorted_elements.sort_by(|a, b| a.id.cmp(&b.id));

                // let mut log = vec![];

                doc.element_tree.traverse(
                    |_idx, element, _| {
                        let hier_level: usize = count_char_occurrences(&element.id, '.');
                        let element_part: String = if hier_level > 0 {
                            get_slice_after_last_occurrence(&element.id, '.').unwrap()
                        } else {
                            element.id.clone()
                        };
                        let element_path: String = if hier_level > 0 {
                            get_slice_after_first_occurrence(&element.id, '.').unwrap_or(element.id.clone())
                        } else {
                            element.id.clone()
                        };
                        // if (hier_level as isize - current_level as isize).abs() > 1 {
                        //     return Err(format!("Hierarchical level difference is too large: {}", element.id).into());
                        // }
                        match hier_level.cmp(&current_level) {
                            std::cmp::Ordering::Greater => {
                                levels.push(1);
                                current_level += 1;
                            }
                            std::cmp::Ordering::Less => {
                                levels.pop();
                                current_level -= 1;
                                levels[current_level] += 1;
                            }
                            std::cmp::Ordering::Equal => {
                                levels[current_level] += 1;
                            }
                        }
                        let mut code = prefix.clone();
                        for level in &levels[1..=current_level] {
                            code.push('.');
                            code.push_str(&level.to_string());
                        }

                        let description = if element.short == element.definition {
                            element.short.clone()
                        } else {
                            format!("{}<br/>{}", element.short, element.definition.replace("\n", "<br/>"))
                        };
                        let element_part_no_x = element_part.replace("[x]", "");
                        write!(
                            writer,
                            "| {} | {} | {} | {} | {} | {}..{} | {}..{} |",
                            code,
                            element_path,
                            camel_to_spaced_pascal(&element_part_no_x),
                            description,
                            reduce_datatypes(&element.datatype),
                            element.min,
                            element.max,
                            element.global_min,
                            element.global_max
                        )
                        .unwrap();
                        if let Some(binding) = &element.binding {
                            write!(writer, " {} |", binding).unwrap();
                        } else {
                            write!(writer, " |").unwrap();
                        }
                        // if let Some(binding_strength) = &element.binding_strength {
                        //     write!(writer, " {} |", binding_strength).unwrap();
                        // } else {
                        //     write!(writer, " |").unwrap();
                        // }
                        if let Some(requirements) = &element.requirements {
                            write!(writer, " {} |", requirements.replace("\n", "<br/>")).unwrap();
                        } else {
                            write!(writer, " |").unwrap();
                        }
                        // if let Some((actor, code, documentation)) = element.obligation.first() {
                        //     write!(writer, " {} ({}) | {} |", actor, code, documentation.replace("\n", "<br/>")).unwrap();
                        // } else {
                        //     write!(writer, " | |").unwrap();
                        // }
                        writeln!(writer).unwrap();
                    },
                    |_, _, _| (),
                    &mut (),
                );
            }
        }
        Commands::PlantUml(args) => {
            // first load all structure definitions into in-memory structs
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
                let mut relations = String::new();

                // let mut _element_number = 0;

                doc.element_tree.traverse(
                    |_idx, element, _| {
                        if let Some(element_part) =
                            get_slice_after_last_occurrence(&element.id, '.')
                            && element.max != "0"
                        {
                            let hier_level = count_char_occurrences(&element.id, '.') * 2;
                            // if the datatype is one of the classes drawn, add a relation instead of a class element
                            // TODO: element is removed from element list if there is one datatype that is among the structure definitions
                            let mut show_this_element = true;
                            if element_part.ends_with("[x]") {
                                let element_part_no_x = element_part.replace("[x]", "");
                                let choice: String = format!("{}{}", doc.id, element_part_no_x);
                                let mut local_relations = String::new();
                                for datatype in element.datatype.iter() {
                                    // TODO: or use a hashmap for faster lookup
                                    // TODO: look also for Reference(X or T)
                                    if docs.iter().any(|d| datatype == &d.id) {
                                        local_relations += &format!(
                                            "{} .. \"**{}**\" : {} >\n",
                                            choice, datatype, element_part_no_x
                                        );
                                        // will hide element if there is just one datatype that is another class in the diagram,
                                        show_this_element = false; // do not show element if it is a choice
                                    }
                                }
                                if !show_this_element {
                                    relations += &format!("<> {}\n", choice);
                                    relations += &format!(
                                        "\"**{}**\" -- \"{}..{}\" {} : {} >\n",
                                        doc.id, element.min, element.max, choice, element_part_no_x
                                    );
                                    relations += &local_relations;
                                }
                            } else {
                                for datatype in element.datatype.iter() {
                                    // TODO: or use a hashmap for faster lookup
                                    // TODO: look also for Reference(X or T)
                                    if docs.iter().any(|d| datatype == &d.id) {
                                        relations += &format!(
                                            "\"**{}**\" -- \"{}..{}\" \"**{}**\" : {} >\n",
                                            doc.id,
                                            element.global_min,
                                            element.global_max,
                                            datatype,
                                            element_part
                                        );
                                        show_this_element = false; // do not show element if datatype is another class in the diagram
                                    }
                                }
                            }

                            if show_this_element && !args.elements_hide {
                                write!(
                                    writer,
                                    "{:>hier_level$}|_ {} : {}",
                                    "",
                                    element_part,
                                    reduce_datatypes(&element.datatype)
                                )
                                .unwrap();
                                if !args.cardinality_hide {
                                    write!(writer, " [{}..{}]", element.min, element.max).unwrap();
                                }
                                writeln!(writer).unwrap();
                            }
                        }
                    },
                    |_, _, _| (),
                    &mut (),
                );

                writeln!(writer, "}}").unwrap();

                write!(writer, "{}", relations).unwrap();
            }

            writeln!(writer, "@enduml")?;
        }
        Commands::Mindmap(mindmap_args) => {
            // first load all structure definitions into in-memory structs
            let docs = load_structure_definition_files(&mindmap_args.common.files)?;
            for doc in docs.iter() {
                println!("processing: {}", doc.id);
                let output = File::create(format!("{}_mindmap.plantuml", doc.id))?;
                let mut writer = BufWriter::new(output); // Create a buffered writer

                writeln!(
                    writer,
                    "@startmindmap\nskinparam topurl StructureDefinition-\n\n* **[[{}.html {}]]**",
                    doc.id, doc.id
                )?;

                doc.element_tree.traverse(
                    |_idx, element, _| {
                        if let Some(element_part) =
                            get_slice_after_last_occurrence(&element.id, '.')
                            && element.max != "0"
                        {
                            let hier_level = count_char_occurrences(&element.id, '.') + 1;

                            writeln!(
                                writer,
                                "{}{} {}",
                                "*".repeat(hier_level),
                                if hier_level > mindmap_args.box_level {
                                    "_"
                                } else {
                                    ""
                                },
                                camel_to_spaced_pascal(&element_part.replace("[x]", ""))
                            )
                            .unwrap();
                        }
                    },
                    |_, _, _| (),
                    &mut (),
                );

                writeln!(writer, "@endmindmap")?;
            }
        }
        Commands::Obligations(args) => {
            let actors = if let Some(actors_folder) = args.actors_folder {
                load_actor_files(&actors_folder)?
            } else {
                HashMap::<String, String>::new()
            };

            let docs = load_structure_definition_files(&args.common.files)?;
            for doc  in docs.iter() {
                println!("processing: {}", doc.id);
                let output = File::create(format!("{}.md", doc.id))?;
                let mut writer = BufWriter::new(output); // Create a buffered writer

                writeln!(writer, "# {}", doc.id)?;

                // table: path, element, actor1: [code, documentation], actor2: [code, documentation], ...
                let mut table = Vec::<(String, String, HashMap<String, Vec<(String, String)>>)>::new();
                let mut unique_actors = HashSet::<String>::new();

                doc.element_tree.traverse(
                    |_idx, element, _| {
                        let hier_level: usize = count_char_occurrences(&element.id, '.');
                        let element_part: String = if hier_level > 0 {
                            get_slice_after_last_occurrence(&element.id, '.').unwrap()
                        } else {
                            element.id.clone()
                        };
                        let element_path: String = if hier_level > 0 {
                            get_slice_after_first_occurrence(&element.id, '.').unwrap_or(element.id.clone())
                        } else {
                            element.id.clone()
                        };

                        if !element.obligation.is_empty() {
                            table.push((
                                element_path.clone(),
                                element_part.clone(),
                                HashMap::<String, Vec<(String, String)>>::new(),
                            ));

                            for obligation in &element.obligation {
                                let hash =
                                    &mut table.last_mut().unwrap().2;
                                let actor = obligation.0.clone();
                                unique_actors.insert(actor.clone());
                                let code = obligation.1.clone();
                                let documentation = obligation.2.clone();
                                let codes = hash.entry(actor).or_default();
                                codes.push((code, documentation));
                            }
                        }
                    },
                    |_, _, _| (),
                    &mut (),
                );

                let no_of_actors = unique_actors.len();

                write!(writer, "| Path | Element ")?;
                for actor in unique_actors.iter() {
                    let actor_name = if let Some(name) = actors.get(actor) {
                        name.clone()
                    } else {
                        get_slice_after_last_occurrence(actor, '/').ok_or("Wrong actor URL")?
                    };
                    write!(writer, "| {} ", actor_name)?;
                }
                writeln!(writer, "|")?;
                write!(writer, "| --- | --- ")?;
                for _ in 0..no_of_actors {
                    write!(writer, "| --- ")?;
                }
                writeln!(writer, "|")?;

                for (element_path, element_part, hash) in table.iter() {
                    write!(writer, "| {} ", element_path)?;
                    let element_part_no_x = element_part.replace("[x]", "");
                    write!(writer, "| {} ", camel_to_spaced_pascal(&element_part_no_x))?;
                    for actor in unique_actors.iter() {
                        if let Some(codes) = hash.get(actor) {
                            write!(writer, 
                                "| <table>{}</table> ", 
                                codes.iter().map(|(code, documentation)| { 
                                    if documentation.is_empty() {
                                        format!("<tr><td>{}</td><td></td></tr>", code)
                                    } else {
                                        format!("<tr><td>{}</td><td>{}</td></tr>", code, documentation)
                                    }
                                }).collect::<Vec<_>>().join("")
                            )?;
                        } else {
                            write!(writer, "| ")?;
                        }
                    }
                    writeln!(writer, "|")?;
                }
            }
        }
    }

    Ok(())
}

fn load_actor_files(path: &PathBuf) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
    let mut actors = HashMap::<String, String>::new();
    let paths = std::fs::read_dir(path)?
        .filter_map(|res| res.ok())
        .filter(|e| e.path().file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with("ActorDefinition-") && n.ends_with(".json")));
    for entry in paths {
        let path = entry.path();
        if path.is_file() {
            let doc = load_json_from_file(&path)?;
            let id = doc["url"].as_str().ok_or("Missing id")?.to_string();
            let name = doc["name"].as_str().ok_or("Missing name")?.to_string();
            actors.insert(id, name);
        }
    }
    Ok(actors)
}
 
fn load_structure_definition_files(
    files: &[PathBuf],
) -> Result<Vec<StructureDefTreeInfo>, Box<dyn std::error::Error>> {
    let mut docs = Vec::<StructureDefTreeInfo>::new();
    for file in files.iter() {
        match load_single_structure_definition_file_into_tree(file) {
            Ok(doc_info) => {
                docs.push(doc_info);
            }
            Err(e) => {
                println!("Error reading file '{}': {}", file.display(), e);
            }
        }
    }
    Ok(docs)
}

fn load_single_structure_definition_file_into_tree(
    file: &PathBuf,
) -> Result<StructureDefTreeInfo, Box<dyn std::error::Error>> {
    let doc = load_json_from_file(file)?;
    let id = doc["id"].as_str().ok_or("Missing id")?;
    let snapshot = doc["snapshot"]["element"]
        .as_array()
        .ok_or("Missing snapshot")?;
    let mut element_tree: Tree<ElementInfo> = Tree::new();
    // let mut elements = Vec::<ElementInfo>::new();
    for element in snapshot.iter() {
        let element_id = element["id"].as_str().ok_or("Missing element id")?;
        let parent_id = element_id
            .rfind('.')
            .map(|last_index| &element_id[..last_index]);
        let parent_node = if let Some(pid) = parent_id {
            element_tree.find_first(|e| e.id == pid)
        } else {
            None
        };
        let short = element["short"]
            .as_str()
            .ok_or("Missing short description")?
            .to_string();
        let definition = element["definition"]
            .as_str()
            .ok_or("Missing definition")?
            .to_string();
        let requirements = element["requirements"].as_str().map(|s| s.to_string());

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
                        // TODO: does not distinguish between Reference and direct datatype
                        if let Some(profiles) = dt["targetProfile"].as_array() {
                            for profile_value in profiles {
                                if let Some(profile) = profile_value.as_str() {
                                    let profile = profile.to_string();
                                    if let Some(end) =
                                        get_slice_after_last_occurrence(&profile, '/')
                                    {
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

        let mut obligation = Vec::<(String, String, String)>::new();
        if let Some(ext_array) = element["extension"].as_array() {
            for ext in ext_array {
                if ext["url"].as_str() == Some("http://hl7.org/fhir/StructureDefinition/obligation")
                {
                    let mut code = String::new();
                    let mut actor = String::new();
                    let mut documentation = String::new();
                    if let Some(ext2_array) = ext["extension"].as_array() {
                        for ext2 in ext2_array {
                            if ext2["url"].as_str() == Some("code") {
                                if let Some(value) = ext2["valueCode"].as_str() {
                                    code = value.to_string();
                                }
                            } else if ext2["url"].as_str() == Some("actor") {
                                if let Some(value) = ext2["valueCanonical"].as_str() {
                                    actor = value.to_string();
                                }
                            } else if ext2["url"].as_str() == Some("documentation")
                                && let Some(value) = ext2["valueMarkdown"].as_str() {
                                    documentation = value.to_string();
                                }
                        }
                    }
                    if !code.is_empty() && !actor.is_empty() {
                        obligation.push((actor, code, documentation));
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

        let max = element["max"]
            .as_str()
            .ok_or("Missing max cardinality")?
            .to_string();

        let mut global_min = min.clone();
        let mut global_max: String = max.clone();
        let mut parent_iterator = parent_node;
        while let Some(p) = parent_iterator {
            if p == 0 {
                break;
            }
            if let Some(e) = element_tree.get_data_of(p) {
                let parent_min = &e.min;
                if global_min == "0" {
                    // do nothing
                } else {
                    let res = parent_min.cmp(&global_min);
                    if res == std::cmp::Ordering::Less {
                        global_min = parent_min.clone();
                    };
                }

                let parent_max = &e.max;
                if global_max == "*" {
                    // do nothing
                } else if parent_max == "*" {
                    global_max = parent_max.clone();
                } else {
                    let res = parent_max.cmp(&global_max);
                    if res == std::cmp::Ordering::Greater {
                        global_max = parent_max.clone();
                    };
                }
            }
            parent_iterator = element_tree.get_parent_of(parent_iterator);
        }

        let binding = element["binding"]["description"]
            .as_str()
            .map(|s| s.to_string());
        let binding_strength = element["binding"]["strength"]
            .as_str()
            .map(|s| s.to_string());

        if let Some(parent) = parent_node {
            element_tree.add_child(
                parent,
                ElementInfo {
                    id: element_id.to_string(),
                    short: short.clone(),
                    definition: definition.clone(),
                    datatype: datatype.clone(),
                    min: min.clone(),
                    max: max.to_string(),
                    global_min: global_min.clone(),
                    global_max: global_max.clone(),
                    binding: binding.clone(),
                    binding_strength: binding_strength.clone(),
                    obligation: obligation.clone(),
                    requirements: requirements.clone(),
                },
            );
        } else {
            element_tree.add_node(ElementInfo {
                id: element_id.to_string(),
                short: short.clone(),
                definition: definition.clone(),
                datatype: datatype.clone(),
                min: min.clone(),
                max: max.to_string(),
                global_min: global_min.clone(),
                global_max: global_max.clone(),
                binding: binding.clone(),
                binding_strength: binding_strength.clone(),
                obligation: obligation.clone(),
                requirements: requirements.clone(),
            });
        }
    }
    Ok(StructureDefTreeInfo {
        id: id.to_string(),
        element_tree,
    })
}
