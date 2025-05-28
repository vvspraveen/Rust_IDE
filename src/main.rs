// main.rs
use eframe::{egui, App};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;
use walkdir::WalkDir;
use reqwest::Client;
use futures_util::stream::StreamExt;
use egui::{FontDefinitions, FontFamily};

////////////////////////////////////////////////////////
use syntect::easy::HighlightLines;
use syntect::highlighting::{ThemeSet, Style};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

////////////////////////////////////////////////////////

#[derive(Debug)]
enum FileNode {
    File(PathBuf),
    Directory(String, Vec<FileNode>, PathBuf),
}

#[derive(Default)]
struct FileOpState {
    target: Option<PathBuf>,
    action: Option<String>,
    input: String,
}

struct MyApp {
    prompt_input: String,
    output: Arc<Mutex<String>>, 
    running: Arc<Mutex<bool>>,
    rt: Runtime,
    theme_dark: bool,
    auto_scroll: bool,
    phase: Arc<Mutex<String>>,
    selected_file: Arc<Mutex<Option<PathBuf>>>,
    selected_file_content: Arc<Mutex<String>>,
    file_op_state: Arc<Mutex<FileOpState>>,
    open_tabs: Vec<(PathBuf, String)>,
    current_tab: Option<usize>,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            prompt_input: String::new(),
            output: Arc::new(Mutex::new(String::new())),
            running: Arc::new(Mutex::new(false)),
            rt: Runtime::new().unwrap(),
            theme_dark: true,
            auto_scroll: true,
            phase: Arc::new(Mutex::new("Idle".to_string())),
            selected_file: Arc::new(Mutex::new(None)),
            selected_file_content: Arc::new(Mutex::new(String::new())),
            file_op_state: Arc::new(Mutex::new(FileOpState::default())),
            open_tabs: Vec::new(),
            current_tab: None,
        }
    }
}

impl App for MyApp {

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        static INIT: std::sync::Once = std::sync::Once::new();
        INIT.call_once(|| self.configure_fonts(ctx));

        if self.theme_dark {
            ctx.set_visuals(egui::Visuals::dark());
        } else {
            ctx.set_visuals(egui::Visuals::light());
        }

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("🧠 Prompt:");
                ui.text_edit_singleline(&mut self.prompt_input);
                if ui.button("▶️ Run").clicked() && !*self.running.lock().unwrap() {
                    self.run_task(ctx.clone());
                }
                // if ui.button("💾 Save Output").clicked() {
                //     let contents = self.output.lock().unwrap().clone();
                //     let _ = fs::write("output.txt", contents);
                // }
                if ui.button("💾 Save Output to file").clicked() {
                        let contents = self.output.lock().unwrap().clone();
                        let path = PathBuf::from("output.txt");
                        match fs::write(&path, &contents) {
                            Ok(_) => {
                                *self.phase.lock().unwrap() = "✅ Output saved to output.txt".to_string();
                                // open in tab
                                self.open_tabs.push((path.clone(), contents));
                                self.current_tab = Some(self.open_tabs.len() - 1);
                            }
                            Err(e) => {
                                *self.phase.lock().unwrap() = format!("❌ Failed to save: {e}");
                            }
                        }
                    }

                if ui.button("🔧 Build & Run").clicked() && !*self.running.lock().unwrap() {
                    self.run_project(ctx.clone());
                }
            });
        });

        egui::SidePanel::left("file_tree").resizable(true).default_width(200.0).show(ctx, |ui| {
            ui.heading("📂 Project Files");
            let root = PathBuf::from("Rust_Project");
            let tree = self.build_tree(&root);
            self.render_tree(ui, &tree, ctx);
        });

        egui::SidePanel::right("settings_panel").resizable(true).show(ctx, |ui| {
            ui.heading("⚙️ Settings");
            let changed = ui.checkbox(&mut self.theme_dark, "Dark Theme").changed();
            if changed {
                if self.theme_dark {
                    ctx.set_visuals(egui::Visuals::dark());
                } else {
                    ctx.set_visuals(egui::Visuals::light());
                }
            }
            ui.checkbox(&mut self.auto_scroll, "Auto-Scroll Output");
            ui.label(format!("Status: {}", self.phase.lock().unwrap()));
        });

        egui::CentralPanel::default().show(ctx, |ui| {

    ui.horizontal(|ui| {
    let mut to_close: Option<usize> = None;

    for (i, (path, _)) in self.open_tabs.iter().enumerate() {
        let filename = path.file_name().unwrap().to_string_lossy();

        ui.horizontal(|ui| {
            let selected = Some(i) == self.current_tab;
            if ui.selectable_label(selected, filename.clone()).clicked() {
                self.current_tab = Some(i);
            }
            if ui.button("✕").on_hover_text("Close tab").clicked() {
                to_close = Some(i);
            }
        });
    }

    if let Some(idx) = to_close {
        self.open_tabs.remove(idx);
        if let Some(current) = self.current_tab {
            if current == idx {
                self.current_tab = if self.open_tabs.is_empty() {
                    None
                } else if idx >= self.open_tabs.len() {
                    Some(self.open_tabs.len() - 1)
                } else {
                    Some(idx)
                };
            } else if current > idx {
                self.current_tab = Some(current - 1);
            }
        }
    }
});




if let Some(index) = self.current_tab {
    if let Some((path, content)) = self.open_tabs.get_mut(index) {
        let path = path.clone();
        let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

        ui.separator();
        ui.heading(format!("📄 Editing: {}", path.display()));

        let mut save_clicked = false;

        egui::ScrollArea::both().id_source("editor_scroll").show(ui, |ui| {
            ui.horizontal(|ui| {
                // Gutter with line numbers
                ui.vertical(|ui| {
                    for (i, _) in lines.iter().enumerate() {
                        ui.label(
                            egui::RichText::new(format!("{:>3}", i + 1))
                                .monospace()
                                .small(),
                        );
                    }
                });

                // Editor Area
                ui.vertical(|ui| {
                    ui.add_sized(
                        [ui.available_width(), 20.0 * lines.len().max(1) as f32],
                        egui::TextEdit::multiline(content)
                            .font(egui::TextStyle::Monospace)
                            .code_editor()
                            .desired_rows(lines.len().max(1)),
                    );
                    //render_highlighted_code(ui, content);

                    ui.add_space(8.0);
                    save_clicked = ui.button("💾 Save File").clicked();
                });
            });
        });

        if save_clicked {
            let _ = fs::write(&path, &*content);

            if path.file_name().map_or(false, |f| f == "main.rs")
                && path.ends_with(Path::new("Rust_Project/src/main.rs"))
            {
                let code = content.clone();
                let mut cargo_toml = String::from(
                    "[package]\nname = \"generated_project\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\n",
                );

                if code.contains("tokio::") {
                    cargo_toml.push_str("tokio = { version = \"1\", features = [\"full\"] }\n");
                }
                if code.contains("serde") {
                    cargo_toml.push_str("serde = { version = \"1\", features = [\"derive\"] }\n");
                }
                if code.contains("chrono") {
                    cargo_toml.push_str("chrono = \"0.4\"\n");
                }
                if code.contains("reqwest") {
                    cargo_toml.push_str("reqwest = { version = \"0.11\", features = [\"json\"] }\n");
                }
                if code.contains("rand") {
                    cargo_toml.push_str("rand = \"0.8\"\n");
                }

                let _ = fs::write("Rust_Project/Cargo.toml", cargo_toml);

                // 🔁 Refresh Cargo.toml if it's open
                if let Some(cargo_tab) = self.open_tabs.iter_mut().find(|(p, _)| {
                    p.file_name().map_or(false, |f| f == "Cargo.toml")
                }) {
                    let updated = fs::read_to_string("Rust_Project/Cargo.toml").unwrap_or_default();
                    cargo_tab.1 = updated;
                }
            }
        }
    }
}

        });


        egui::TopBottomPanel::bottom("terminal_output").resizable(true).show(ctx, |ui| {
            ui.heading("🧾 Terminal Output");
            egui::ScrollArea::vertical().show(ui, |ui| {
                let mut text = self.output.lock().unwrap().clone();
                //ui.text_edit_multiline(&mut text);
                ui.add_sized(ui.available_size(), egui::TextEdit::multiline(&mut text).code_editor());
            });
        });

        self.handle_file_ops(ctx);
    }
}

impl MyApp {

    

    fn configure_fonts(&self, ctx: &egui::Context) {
        let fonts = FontDefinitions::default();
        ctx.set_fonts(fonts);
    }

    fn run_task(&mut self, ctx: egui::Context) {
    let prompt = self.prompt_input.clone();
    let output_clone = self.output.clone();
    let running_clone = self.running.clone();
    let ctx_clone = ctx.clone();
    let phase_ref = self.phase.clone();

    *running_clone.lock().unwrap() = true;
    *phase_ref.lock().unwrap() = "🔁 Sending prompt".to_string();

    self.rt.spawn(async move {
        let client = reqwest::Client::new();
        let res = client
            .post("http://localhost:3000/prompt")
            .json(&serde_json::json!({ "prompt": prompt }))
            .send()
            .await;

        if let Ok(_) = res {
            *phase_ref.lock().unwrap() = "📰 Streaming response...".to_string();

            if let Ok(response) = client.get("http://localhost:3000/prompt-stream").send().await {
                let mut final_output = String::new();
                let mut stream = response.bytes_stream();

                while let Some(Ok(chunk)) = stream.next().await {
                    let part = String::from_utf8_lossy(&chunk);
                    for line in part.lines() {
                        if let Some(stripped) = line.strip_prefix("data:") {
                            if stripped == "__DOWNLOAD__" { continue; }
                            final_output.push_str(stripped);
                        }
                    }
                    *output_clone.lock().unwrap() = final_output.clone();
                    ctx_clone.request_repaint();
                }

                let _ = fs::create_dir_all("Rust_Project/src");
                let _ = fs::write("Rust_Project/src/main.rs", &final_output);
                *phase_ref.lock().unwrap() = "✅ Done. Written to main.rs".to_string();
            }
        }

        *running_clone.lock().unwrap() = false;
        ctx_clone.request_repaint();
    });
}

fn run_project(&self, ctx: egui::Context) {
    let output = self.output.clone();
    let running = self.running.clone();
    *running.lock().unwrap() = true;
    *output.lock().unwrap() = "🔧 Running cargo...".to_string();

    std::thread::spawn(move || {
        let result = std::process::Command::new("cargo")
            .arg("run")
            .current_dir("Rust_Project")
            .output();

        *output.lock().unwrap() = match result {
            Ok(out) => format!(
                "{}\n{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            ),
            Err(e) => format!("❌ Failed: {e}"),
        };

        *running.lock().unwrap() = false;
        ctx.request_repaint();
    });
}


    fn build_tree(&self, root: &PathBuf) -> FileNode {
        fn insert(tree: &mut FileNode, base: &Path, path: &Path) {
            if let FileNode::Directory(_, children, _) = tree {
                let rel_path = path.strip_prefix(base).unwrap();
                let mut parts = rel_path.components();
                if let Some(first) = parts.next() {
                    let name = first.as_os_str().to_string_lossy().to_string();
                    let next_path = base.join(&name);
                    if parts.clone().next().is_none() {
                        children.push(if path.is_dir() {
                            FileNode::Directory(name, vec![], path.to_path_buf())
                        } else {
                            FileNode::File(path.to_path_buf())
                        });
                    } else {
                        let mut found = false;
                        for child in children.iter_mut() {
                            if let FileNode::Directory(child_name, _, _) = child {
                                if *child_name == name {
                                    insert(child, &next_path, path);
                                    found = true;
                                    break;
                                }
                            }
                        }
                        if !found {
                            let mut new_dir = FileNode::Directory(name.clone(), vec![], next_path.clone());
                            insert(&mut new_dir, &next_path, path);
                            children.push(new_dir);
                        }
                    }
                }
            }
        }

        let mut root_node = FileNode::Directory("Rust_Project".to_string(), vec![], root.clone());
        for entry in WalkDir::new(root).into_iter().filter_map(Result::ok) {
            insert(&mut root_node, root, entry.path());
        }
        root_node
    }

    fn render_tree(&mut self, ui: &mut egui::Ui, node: &FileNode, ctx: &egui::Context) {
        match node {
            FileNode::File(path) => {
                let label = match path.extension().and_then(|e| e.to_str()) {
                    Some("rs") => format!("🦀 {}", path.file_name().unwrap().to_string_lossy()),
                    Some("toml") => format!("📦 {}", path.file_name().unwrap().to_string_lossy()),
                    _ => path.file_name().unwrap().to_string_lossy().to_string(),
                };

                let response = ui.button(label);
                if response.clicked() {
                    let path = path.clone();
                    let existing_index = self.open_tabs.iter().position(|(p, _)| *p == path);

                    if let Some(index) = existing_index {
                        self.current_tab = Some(index);
                    } else {
                        let content = fs::read_to_string(&path).unwrap_or_default();
                        self.open_tabs.push((path.clone(), content));
                        self.current_tab = Some(self.open_tabs.len() - 1);
                    }
                }

                // if response.clicked() {
                //     let content = fs::read_to_string(path).unwrap_or_default();
                //     *self.selected_file.lock().unwrap() = Some(path.clone());
                //     *self.selected_file_content.lock().unwrap() = content;
                // }

                response.context_menu(|ui| {
                    if ui.button("✏️ Rename").clicked() {
                        *self.file_op_state.lock().unwrap() = FileOpState {
                            target: Some(path.clone()),
                            action: Some("rename".to_string()),
                            input: path.file_name().unwrap().to_string_lossy().to_string(),
                        };
                        ui.close_menu();
                    }
                    if ui.button("🗑 Delete").clicked() {
                        let _ = fs::remove_file(path);
                        ui.close_menu();
                    }
                });
            }
            FileNode::Directory(name, children, path) => {
                let response = ui.collapsing(name, |ui| {
                    for child in children {
                        self.render_tree(ui, child, ctx);
                    }
                });

                response.header_response.context_menu(|ui| {
                    if ui.button("📄 New File").clicked() {
                        *self.file_op_state.lock().unwrap() = FileOpState {
                            target: Some(path.clone()),
                            action: Some("new_file".to_string()),
                            input: String::new(),
                        };
                        ui.close_menu();
                    }
                    if ui.button("📁 New Folder").clicked() {
                        *self.file_op_state.lock().unwrap() = FileOpState {
                            target: Some(path.clone()),
                            action: Some("new_folder".to_string()),
                            input: String::new(),
                        };
                        ui.close_menu();
                    }
                    if ui.button("✏️ Rename").clicked() {
                        *self.file_op_state.lock().unwrap() = FileOpState {
                            target: Some(path.clone()),
                            action: Some("rename".to_string()),
                            input: path.file_name().unwrap().to_string_lossy().to_string(),
                        };
                        ui.close_menu();
                    }
                    if ui.button("🗑 Delete Folder").clicked() {
                        let _ = fs::remove_dir_all(path);
                        ui.close_menu();
                    }
                });
            }
        }
    }

    fn handle_file_ops(&self, ctx: &egui::Context) {
        let mut state = self.file_op_state.lock().unwrap();
        let target = state.target.clone();
        let action = state.action.clone();
        let input = state.input.clone();

        if let (Some(target), Some(action)) = (target, action) {
            egui::Window::new("File Operation")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label(format!("Target: {}", target.display()));
                    ui.text_edit_singleline(&mut state.input);

                    ui.horizontal(|ui| {
                        if ui.button("OK").clicked() {
                            let name = state.input.trim();
                            if !name.is_empty() {
                                match action.as_str() {
                                    "new_file" => {
                                        let _ = fs::write(target.join(name), "");
                                    }
                                    "new_folder" => {
                                        let _ = fs::create_dir_all(target.join(name));
                                    }
                                    "rename" => {
                                        if let Some(parent) = target.parent() {
                                            let _ = fs::rename(&target, parent.join(name));
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            *state = FileOpState::default();
                        }
                        if ui.button("Cancel").clicked() {
                            *state = FileOpState::default();
                        }
                    });
                });
        }
    }
}


// fn render_highlighted_code(ui: &mut egui::Ui, code: &str) {
//     let ps = SyntaxSet::load_defaults_newlines();
//     let ts = ThemeSet::load_defaults();
//     let syntax = ps.find_syntax_by_extension("rs").unwrap();
//     let mut h = HighlightLines::new(syntax, &ts.themes["base16-ocean.dark"]);

//     egui::ScrollArea::both().id_source("highlighted_code_scroll").show(ui, |ui| {
//         for line in LinesWithEndings::from(code) {
//             let ranges: Vec<(Style, &str)> = h.highlight_line(line, &ps).unwrap();
//             let mut layout_job = egui::text::LayoutJob::default();

//             for (style, text) in ranges {
//                 let color = egui::Color32::from_rgb(style.foreground.r, style.foreground.g, style.foreground.b);
//                 layout_job.append(
//                     text,
//                     0.0,
//                     egui::TextFormat {
//                         font_id: egui::FontId::monospace(14.0), 
//                         color,
//                         ..Default::default()
//                     },
//                 );
//             }

//             ui.label(layout_job);
//         }
//     });
// }

fn main() {
    let options = eframe::NativeOptions::default();
    let _ = eframe::run_native("Rust IDE", options, Box::new(|_| Box::new(MyApp::default())));
}
