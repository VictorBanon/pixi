use eframe::egui::Ui;
use egui::{Color32, FontFamily, FontId, Stroke, TextStyle};
use pixi::{Db, TodoItem};

const HEADING: &str = "📋 Lista de Tareas";

const INNER_SIZE_X: f32 = 500.;
// const INNER_SIZE_Y: f32 = 700.;
const INNER_SIZE_X_MIN: f32 = 400.;
const INNER_SIZE_Y_MIN: f32 = 300.;
const INNER_SIZE_X_MAX: f32 = 1000.;
const INNER_SIZE_Y_MAX: f32 = 1000.;

// cf. render_header
const HEADER_Y: f32 = 43.;
// cf. render_stats
const STATS_Y: f32 = 72.;
// cf. render_task_item
const TASK_Y: f32 = 46.;

// The above INNER_SIZE_Y get overwriten by resizing and are sort of useless

fn main() -> Result<(), eframe::Error> {
    let db = Db::new("tareas.db").unwrap();
    let num_tareas = db.cargar_tareas().unwrap().len();
    let needed_height = MyApp::needed_height(num_tareas);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([INNER_SIZE_X, needed_height])
            .with_min_inner_size([INNER_SIZE_X_MIN, INNER_SIZE_Y_MIN])
            .with_max_inner_size([INNER_SIZE_X_MAX, INNER_SIZE_Y_MAX]),
        ..Default::default()
    };

    eframe::run_native(
        HEADING,
        options,
        // Box::new(|_cc| Ok(Box::<MyApp>::default())),
        Box::new(|cc| Ok(app_creator(cc))),
    )
}

fn app_creator(cc: &eframe::CreationContext<'_>) -> Box<dyn eframe::App> {
    cc.egui_ctx.set_visuals(egui::Visuals::dark());

    // Fonts
    {
        let mut style = (*cc.egui_ctx.style()).clone();
        style.text_styles = [
            (
                TextStyle::Heading,
                FontId::new(24.0, FontFamily::Proportional),
            ),
            (TextStyle::Body, FontId::new(16.0, FontFamily::Proportional)),
            (
                TextStyle::Button,
                FontId::new(16.0, FontFamily::Proportional),
            ),
            (
                TextStyle::Small,
                FontId::new(12.0, FontFamily::Proportional),
            ),
            (
                TextStyle::Monospace,
                FontId::new(14.0, FontFamily::Monospace),
            ),
        ]
        .into();
        cc.egui_ctx.set_style(style);
    }

    let app = MyApp::default();
    Box::new(app)
}

struct MyApp {
    db: Db,
    todos: Vec<TodoItem>,
    nueva_tarea: String,
    drag_index: Option<usize>,
    editing_index: Option<usize>,
    edit_text: String,
}

impl Default for MyApp {
    fn default() -> Self {
        let db = Db::new("tareas.db").unwrap();
        let todos = db.cargar_tareas().unwrap_or_else(|_| Vec::new());
        Self {
            db,
            todos,
            nueva_tarea: String::new(),
            drag_index: None,
            editing_index: None,
            edit_text: String::new(),
        }
    }
}

impl MyApp {
    fn todo_at(&self, idx: usize) -> &TodoItem {
        &self.todos[idx]
    }

    fn needed_height(num_tareas: usize) -> f32 {
        let task_height = TASK_Y;
        let header_height = HEADER_Y;
        let stats_height = STATS_Y;
        let margin = 90.;
        let needed_height =
            header_height + (num_tareas as f32 * task_height) + stats_height + margin;
        eprintln!("{}", needed_height);
        needed_height
    }

    // Y := 32 + 5 + 1 + 5 = 43px
    fn render_header(&self, ui: &mut egui::Ui) {
        ui.heading(HEADING);
        ui.add_space(5.0);
        ui.separator();
        ui.add_space(5.0);
    }

    fn render_add_task(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Nueva tarea:");
            let text_edit = ui.text_edit_singleline(&mut self.nueva_tarea);

            let should_add = (text_edit.lost_focus()
                && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                || ui.button("➕ Agregar").clicked();

            if should_add && !self.nueva_tarea.trim().is_empty() {
                if self.db.agregar_tarea(&self.nueva_tarea).is_ok() {
                    self.nueva_tarea.clear();
                    self.reload_tasks();
                }
            }
        });

        ui.add_space(5.0);
        ui.separator();
        ui.add_space(5.0);
    }

    // Y :=
    // Drag icon: 18 px
    // Text: 16 px
    // Frame vertical padding: 12 px
    // Extra spacing (maybe ~3–6 px inside layouts)
    // ~~ 46
    fn render_task_item(&mut self, ui: &mut egui::Ui, idx: usize) -> bool {
        let mut should_delete = false;

        let item_id = egui::Id::new("task").with(idx);
        let hover_id = egui::Id::new("task_hover").with(idx);
        let is_being_dragged = self.drag_index == Some(idx);
        let is_editing = self.editing_index == Some(idx);
        let timer_active = self.todo_at(idx).temporizador_activo();

        // Frame con fondo para la tarea - colores para tema oscuro
        let frame = if is_being_dragged {
            egui::Frame::none()
                .fill(Color32::from_rgba_unmultiplied(70, 130, 180, 80))
                .stroke(Stroke::new(2.0, Color32::from_rgb(100, 200, 255)))
                .rounding(5.0)
                .inner_margin(egui::Margin::same(6.0))
        } else if timer_active {
            egui::Frame::none()
                .fill(Color32::from_rgb(40, 60, 40))
                .stroke(Stroke::new(1.5, Color32::from_rgb(100, 200, 100)))
                .rounding(5.0)
                .inner_margin(egui::Margin::same(6.0))
        } else {
            egui::Frame::none()
                .fill(Color32::from_gray(40))
                .rounding(5.0)
                .inner_margin(egui::Margin::same(6.0))
        };

        let frame_response = frame.show(ui, |ui| {
            ui.horizontal(|ui| {
                // Ícono de arrastre más visible (solo si no está editando)
                if !is_editing {
                    ui.vertical(|ui| {
                        ui.add_space(2.0);
                        let drag_label = egui::RichText::new("⣿")
                            .size(18.0)
                            .color(Color32::from_gray(150));
                        let drag_icon = ui.label(drag_label);

                        // Detectar drag en el ícono
                        let sense = egui::Sense::click_and_drag();
                        let drag_response = ui.interact(drag_icon.rect, item_id, sense);

                        if drag_response.hovered() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
                        }

                        if drag_response.drag_started() {
                            self.drag_index = Some(idx);
                        }

                        if drag_response.dragged() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                        }
                    });

                    ui.add_space(4.0);
                }

                if is_editing {
                    // Modo edición
                    let should_save = ui.button("💾").clicked();
                    let should_cancel = ui.button("❌").clicked();
                    ui.text_edit_singleline(&mut self.edit_text);

                    if should_save {
                        let new_text = self.edit_text.clone();
                        if !new_text.trim().is_empty() {
                            let todo = &mut self.todos[idx];
                            let todo_id = todo.id;
                            if self.db.actualizar_descripcion(todo_id, &new_text).is_ok() {
                                todo.text = new_text;
                            }
                            self.editing_index = None;
                            self.edit_text.clear();
                        }
                    }

                    if should_cancel {
                        self.editing_index = None;
                        self.edit_text.clear();
                    }
                } else {
                    // Modo normal
                    let todo = &mut self.todos[idx];
                    let checked_before = todo.checked;

                    ui.checkbox(&mut todo.checked, "");

                    // add text label
                    {
                        // Reserve space on the right for the controls (refresh/delete/etc.)
                        let reserved_for_controls = 220.0_f32;
                        let available_for_text =
                            (ui.available_width() - reserved_for_controls).max(40.0);

                        ui.allocate_ui(
                            egui::vec2(
                                available_for_text,
                                ui.text_style_height(&egui::TextStyle::Body),
                            ),
                            |ui| {
                                ui.with_layout(
                                    egui::Layout::left_to_right(egui::Align::Min),
                                    |ui| {
                                        ui.add(egui::Label::new(todo.text.clone()).truncate());
                                    },
                                );
                            },
                        );
                    }

                    if checked_before != todo.checked {
                        let _ = self.db.actualizar_tarea(todo.id, todo.checked);
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        should_delete = self.render_task_controls(ui, idx);
                    });
                }
            });
        });

        // Guardar la posición del rect para detección en tiempo real
        ui.ctx().memory_mut(|mem| {
            mem.data.insert_temp(hover_id, frame_response.response.rect);
        });

        // Detectar hover para reordenar con línea más visible y zona de drop
        if let Some(drag_idx) = self.drag_index {
            if frame_response.response.hovered() && drag_idx != idx {
                let rect = frame_response.response.rect;

                // Fondo semitransparente para toda el área de drop
                ui.painter().rect_filled(
                    rect,
                    5.0,
                    Color32::from_rgba_unmultiplied(100, 200, 255, 30),
                );

                // Línea indicadora de posición de drop más gruesa y visible
                ui.painter().rect_filled(
                    egui::Rect::from_min_size(
                        egui::pos2(rect.left(), rect.top() - 3.0),
                        egui::vec2(rect.width(), 6.0),
                    ),
                    3.0,
                    Color32::from_rgb(100, 200, 255),
                );
            }
        }

        ui.add_space(3.0);

        should_delete
    }

    fn render_task_controls(&mut self, ui: &mut egui::Ui, idx: usize) -> bool {
        let mut should_delete = false;

        if ui.button("✏️").clicked() {
            let todo = self.todo_at(idx);
            self.edit_text = todo.text.clone();
            self.editing_index = Some(idx);
        }

        if ui.button("🗑").clicked() {
            should_delete = true;
        }

        self.render_timer_display(ui, idx);
        self.render_timer_controls(ui, idx);

        if ui.button("🔄").clicked() {
            let todo = &mut self.todos[idx];
            todo.resetear_temporizador(&self.db);
        }

        should_delete
    }

    fn render_timer_display(&self, ui: &mut Ui, idx: usize) {
        let todo = self.todo_at(idx);
        let tiempo_total = todo.tiempo_total();
        let horas = tiempo_total / 3600;
        let minutos = (tiempo_total % 3600) / 60;
        let segundos = tiempo_total % 60;

        ui.label(format!("⏱ {:02}:{:02}:{:02}", horas, minutos, segundos));
    }

    fn render_timer_controls(&mut self, ui: &mut Ui, idx: usize) {
        let todo = &mut self.todos[idx];
        if todo.temporizador_activo() {
            if ui.button("⏸").clicked() {
                todo.pausar_temporizador(&self.db);
            }
        } else {
            if ui.button("▶").clicked() {
                todo.iniciar_temporizador();
            }
        }
    }

    fn render_tasks(&mut self, ui: &mut egui::Ui) {
        let mut tarea_a_eliminar: Option<usize> = None;
        let mut hover_target: Option<usize> = None;

        for idx in 0..self.todos.len() {
            if self.render_task_item(ui, idx) {
                tarea_a_eliminar = Some(idx);
            }
        }

        // Detectar sobre qué tarea está el cursor mientras arrastra
        if let Some(drag_idx) = self.drag_index {
            for idx in 0..self.todos.len() {
                // Obtener el rect de la tarea
                let task_hover_id = egui::Id::new("task_hover").with(idx);
                if let Some(rect) = ui
                    .ctx()
                    .memory(|mem| mem.data.get_temp::<egui::Rect>(task_hover_id))
                {
                    if let Some(hover_pos) = ui.input(|i| i.pointer.hover_pos()) {
                        if rect.contains(hover_pos) {
                            hover_target = Some(idx);
                            break;
                        }
                    }
                }
            }

            // Mover en tiempo real si hay hover sobre otra tarea
            if let Some(target_idx) = hover_target {
                if drag_idx != target_idx {
                    let item = self.todos.remove(drag_idx);
                    self.todos.insert(target_idx, item);
                    // Actualizar el índice de drag a la nueva posición
                    self.drag_index = Some(target_idx);
                }
            }

            // Liberar cuando se suelta el mouse
            if ui.input(|i| i.pointer.any_released()) {
                self.drag_index = None;
            }
        }

        if let Some(idx) = tarea_a_eliminar {
            self.delete_task(idx);
        }
    }

    // Y := 10 + 1 + 5 + 4 * 18 = 72
    fn render_statistics(&mut self, ui: &mut egui::Ui) {
        ui.add_space(10.0);
        ui.separator();
        ui.add_space(5.0);

        let tiempo_total_segundos: i32 = self.todos.iter().map(|t| t.tiempo_total()).sum();

        let horas_total = tiempo_total_segundos / 3600;
        let minutos_total = (tiempo_total_segundos % 3600) / 60;
        let segundos_total = tiempo_total_segundos % 60;

        let total = self.todos.len();
        let completed = self.todos.iter().filter(|t| t.checked).count();
        let pending = total - completed;

        ui.label(format!("📊 Total: {}", total));
        ui.label(format!("✅ Completadas: {}", completed));
        ui.label(format!("⏳ Pendientes: {}", pending));
        ui.label(format!(
            "⏱️ Tiempo total: {:02}:{:02}:{:02}",
            horas_total, minutos_total, segundos_total
        ));

        if ui.button("🔄 Recargar tareas").clicked() {
            self.reload_tasks();
        }
    }

    fn reload_tasks(&mut self) {
        self.todos = self.db.cargar_tareas().unwrap_or_else(|_| Vec::new());
    }

    fn delete_task(&mut self, idx: usize) {
        let tarea_id = self.todos[idx].id;
        if self.db.eliminar_tarea(tarea_id).is_ok() {
            self.todos.remove(idx);
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint();

        egui::CentralPanel::default()
            .frame(egui::Frame::none().inner_margin(egui::Margin::same(8.0)))
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(4.0, 2.0);

                self.render_header(ui);
                self.render_add_task(ui);
                self.render_tasks(ui);
                self.render_statistics(ui);
            });
    }
}
