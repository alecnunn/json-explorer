use eframe::egui;
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Default)]
struct JsonExplorer {
    root_data: Option<Value>,
    current_data: Option<Value>,
    navigation_path: Vec<String>,
    current_file_path: Option<PathBuf>,
    selected_json: String,
    expanded_nodes: HashMap<String, bool>,
    // Display options
    show_node_types: bool,
    show_node_values: bool,
    // additional windows
    show_about_dialog: bool,
}

impl JsonExplorer {
    fn new() -> Self {
        Self {
            selected_json: String::new(),
            show_node_types: false,  // Default to not showing types
            show_node_values: false, // Default to not showing values
            ..Default::default()
        }
    }

    fn load_file(&mut self, path: PathBuf) -> anyhow::Result<()> {
        let content = std::fs::read_to_string(&path)?;
        let data: Value = serde_json::from_str(&content)?;

        self.root_data = Some(data.clone());
        self.current_data = Some(data);
        self.current_file_path = Some(path);
        self.navigation_path.clear();
        self.expanded_nodes.clear();
        self.update_selected_json();

        Ok(())
    }

    fn navigate_to_path(&mut self, path: Vec<String>) {
        if let Some(root) = &self.root_data {
            let mut current = root;
            let mut valid_path = Vec::new();

            for key in path {
                match current {
                    Value::Object(obj) => {
                        if let Some(value) = obj.get(&key) {
                            current = value;
                            valid_path.push(key);
                        } else {
                            break;
                        }
                    }
                    Value::Array(arr) => {
                        if let Ok(index) = key.parse::<usize>() {
                            if let Some(value) = arr.get(index) {
                                current = value;
                                valid_path.push(key);
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                    _ => break,
                }
            }

            self.navigation_path = valid_path;
            self.current_data = Some(current.clone());
            self.update_selected_json();
        }
    }

    fn go_back(&mut self) {
        if !self.navigation_path.is_empty() {
            self.navigation_path.pop();
            self.navigate_to_path(self.navigation_path.clone());
        }
    }

    fn update_selected_json(&mut self) {
        if let Some(data) = &self.current_data {
            self.selected_json = serde_json::to_string_pretty(data)
                .unwrap_or_else(|_| "Error formatting JSON".to_string());
        }
    }

    fn get_current_path_string(&self) -> String {
        if self.navigation_path.is_empty() {
            "Root".to_string()
        } else {
            self.navigation_path.join(" → ")
        }
    }

    fn render_json_tree(&mut self, ui: &mut egui::Ui, value: &Value, key: &str, path: Vec<String>) {
        let node_id = format!("{}_{}", path.join("_"), key);

        match value {
            Value::Object(obj) => {
                let is_expanded = *self.expanded_nodes.get(&node_id).unwrap_or(&false);

                let icon = if is_expanded { "▼" } else { "▶" };

                // Handle empty key for root nodes
                let display_text = if key.is_empty() {
                    if self.show_node_types {
                        format!("{} Root (object, {} items)", icon, obj.len())
                    } else {
                        format!("{} Root", icon)
                    }
                } else if self.show_node_types {
                    format!("{} {} (object, {} items)", icon, key, obj.len())
                } else {
                    format!("{} {}", icon, key)
                };

                let response = ui.selectable_label(false, display_text);

                if response.clicked() {
                    self.expanded_nodes.insert(node_id.clone(), !is_expanded);
                }

                if response.double_clicked() {
                    let mut new_path = path.clone();
                    if !key.is_empty() {
                        new_path.push(key.to_string());
                    }
                    self.navigate_to_path(new_path);
                }

                // Check expansion state again after potential update
                if *self.expanded_nodes.get(&node_id).unwrap_or(&false) {
                    ui.indent(format!("indent_{}", node_id), |ui| {
                        for (k, v) in obj {
                            let mut child_path = path.clone();
                            if !key.is_empty() {
                                child_path.push(key.to_string());
                            }
                            self.render_json_tree(ui, v, k, child_path);
                        }
                    });
                }
            }
            Value::Array(arr) => {
                let is_expanded = *self.expanded_nodes.get(&node_id).unwrap_or(&false);

                let icon = if is_expanded { "▼" } else { "▶" };

                // Handle empty key for root nodes
                let display_text = if key.is_empty() {
                    if self.show_node_types {
                        format!("{} Root (array, {} items)", icon, arr.len())
                    } else {
                        format!("{} Root", icon)
                    }
                } else if self.show_node_types {
                    format!("{} {} (array, {} items)", icon, key, arr.len())
                } else {
                    format!("{} {}", icon, key)
                };

                let response = ui.selectable_label(false, display_text);

                if response.clicked() {
                    self.expanded_nodes.insert(node_id.clone(), !is_expanded);
                }

                if response.double_clicked() {
                    let mut new_path = path.clone();
                    if !key.is_empty() {
                        new_path.push(key.to_string());
                    }
                    self.navigate_to_path(new_path);
                }

                // Check expansion state again after potential update
                if *self.expanded_nodes.get(&node_id).unwrap_or(&false) {
                    ui.indent(format!("indent_{}", node_id), |ui| {
                        for (i, v) in arr.iter().enumerate() {
                            let mut child_path = path.clone();
                            if !key.is_empty() {
                                child_path.push(key.to_string());
                            }
                            self.render_json_tree(ui, v, &format!("[{}]", i), child_path);
                        }
                    });
                }
            }
            _ => {
                let (type_str, value_str) = match value {
                    Value::String(s) => ("string", format!("\"{}\"", s)),
                    Value::Number(n) => ("number", n.to_string()),
                    Value::Bool(b) => ("boolean", b.to_string()),
                    Value::Null => ("null", "null".to_string()),
                    _ => ("unknown", "?".to_string()),
                };

                let display_value = if value_str.len() > 50 {
                    format!("{}...", &value_str[..50])
                } else {
                    value_str
                };

                // Handle empty key for root leaf nodes (shouldn't happen often)
                let display_key = if key.is_empty() { "Root" } else { key };

                let display_text = if self.show_node_types && self.show_node_values {
                    format!("  {} ({}) {}", display_key, type_str, display_value)
                } else if self.show_node_types && !self.show_node_values {
                    format!("  {} ({})", display_key, type_str)
                } else if !self.show_node_types && self.show_node_values {
                    format!("  {} {}", display_key, display_value)
                } else {
                    format!("  {}", display_key)
                };

                let response = ui.selectable_label(false, display_text);

                if response.clicked() {
                    self.selected_json = serde_json::to_string_pretty(value)
                        .unwrap_or_else(|_| "Error formatting JSON".to_string());
                }
            }
        }
    }
}

impl eframe::App for JsonExplorer {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Menu bar
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Load JSON File").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("JSON", &["json"])
                            .pick_file()
                        {
                            if let Err(e) = self.load_file(path) {
                                eprintln!("Error loading file: {}", e);
                            }
                        }
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Exit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });

                ui.menu_button("View", |ui| {
                    ui.checkbox(&mut self.show_node_types, "Show Node Types");
                    ui.checkbox(&mut self.show_node_values, "Show Node Values");
                });

                ui.menu_button("Help", |ui| {
                    if ui.button("About").clicked() {
                        self.show_about_dialog = true;
                        ui.close_menu();
                    }
                })
            });
        });

        egui::TopBottomPanel::top("control_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Load JSON File").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("JSON", &["json"])
                        .pick_file()
                    {
                        if let Err(e) = self.load_file(path) {
                            eprintln!("Error loading file: {}", e);
                        }
                    }
                }

                ui.separator();

                if ui.button("Go Back").clicked() && !self.navigation_path.is_empty() {
                    self.go_back();
                }

                ui.separator();

                ui.label(format!("Path: {}", self.get_current_path_string()));

                if let Some(file_path) = &self.current_file_path {
                    ui.separator();
                    ui.label(format!(
                        "File: {}",
                        file_path.file_name().unwrap_or_default().to_string_lossy()
                    ));
                }
            });
        });

        egui::SidePanel::left("tree_panel")
            .min_width(400.0)
            .frame(egui::Frame::default().inner_margin(egui::Margin::same(8)))
            .show(ctx, |ui| {
                ui.heading("JSON Structure");
                ui.separator();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);
                    if let Some(data) = self.current_data.clone() {
                        self.render_json_tree(ui, &data, "", vec![]);
                    } else {
                        ui.label("No JSON data loaded");
                    }
                });
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::default().inner_margin(egui::Margin::same(8)))
            .show(ctx, |ui| {
                ui.heading("Raw JSON View");
                ui.separator();

                egui::ScrollArea::both().show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut self.selected_json)
                            .desired_width(f32::INFINITY)
                            .desired_rows(30)
                            .frame(false)
                            .code_editor(),
                    );
                });
            });

        if self.show_about_dialog {
            egui::Window::new("About JSON Explorer")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading("JSON Explorer");
                        ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
                        ui.add_space(10.0);

                        ui.label("A simple JSON file explorer built with Rust and egui");
                        ui.add_space(10.0);

                        ui.hyperlink_to(
                            "View on GitHub",
                            "https://github.com/alecnunn/json-explorer",
                        );

                        ui.add_space(15.0);

                        if ui.button("Close").clicked() {
                            self.show_about_dialog = false;
                        }
                    });
                });
        }
    }
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions::default();

    eframe::run_native(
        "JSON Explorer",
        options,
        Box::new(|_| Ok(Box::new(JsonExplorer::new()))),
    )
}
