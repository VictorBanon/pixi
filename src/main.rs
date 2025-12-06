use eframe::egui::Ui;
use egui::{FontFamily, FontId, TextStyle, Color32, Stroke};
use rusqlite::{Connection, Result as SqlResult};
use std::time::Instant;

const HEADING: &str = "📋 Lista de Tareas";

fn main() -> Result<(), eframe::Error> {
    init_database().expect("Error al inicializar la base de datos");

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([500.0, 200.0])
            .with_min_inner_size([400.0, 150.0]),
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
    // Establecer tema oscuro
    cc.egui_ctx.set_visuals(egui::Visuals::dark());
    
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

    Box::<MyApp>::default()
}

fn init_database() -> SqlResult<()> {
    let conn = Connection::open("tareas.db")?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS tareas (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            descripcion TEXT NOT NULL,
            completada INTEGER DEFAULT 0,
            tiempo_acumulado INTEGER DEFAULT 0
        )",
        [],
    )?;

    // Verificar si ya hay datos
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM tareas", [], |row| row.get(0))?;

    // Si no hay datos, insertar tareas de ejemplo
    if count == 0 {
        let tareas_ejemplo = vec![
            "Comprar leche y pan en el supermercado",
            "Llamar al dentista para cita",
            "Revisar correo electrónico importante",
            "Hacer ejercicio 30 minutos",
            "Leer capítulo del libro",
            "Preparar presentación para reunión",
            "Pagar facturas del mes",
            "Organizar escritorio de trabajo",
            "Estudiar Rust y egui",
            "Backup de archivos importantes",
        ];

        for tarea in tareas_ejemplo {
            conn.execute("INSERT INTO tareas (descripcion) VALUES (?1)", [tarea])?;
        }
    }

    Ok(())
}

fn cargar_tareas() -> SqlResult<Vec<TodoItem>> {
    let conn = Connection::open("tareas.db")?;
    let mut stmt =
        conn.prepare("SELECT id, descripcion, completada, tiempo_acumulado FROM tareas")?;

    let tareas = stmt.query_map([], |row| {
        Ok(TodoItem {
            id: row.get(0)?,
            text: row.get(1)?,
            checked: row.get::<_, i32>(2)? != 0,
            tiempo_acumulado: row.get::<_, i32>(3)?,
            temporizador: None,
        })
    })?;

    Ok(tareas.filter_map(|t| t.ok()).collect())
}

fn actualizar_tarea(id: i32, completada: bool) -> SqlResult<()> {
    let conn = Connection::open("tareas.db")?;
    conn.execute(
        "UPDATE tareas SET completada = ?1 WHERE id = ?2",
        [completada as i32, id],
    )?;
    Ok(())
}

fn agregar_tarea(descripcion: &str) -> SqlResult<()> {
    let conn = Connection::open("tareas.db")?;
    conn.execute(
        "INSERT INTO tareas (descripcion) VALUES (?1)",
        [descripcion],
    )?;
    Ok(())
}

fn eliminar_tarea(id: i32) -> SqlResult<()> {
    let conn = Connection::open("tareas.db")?;
    conn.execute("DELETE FROM tareas WHERE id = ?1", [id])?;
    Ok(())
}

fn actualizar_tiempo(id: i32, tiempo: i32) -> SqlResult<()> {
    let conn = Connection::open("tareas.db")?;
    conn.execute(
        "UPDATE tareas SET tiempo_acumulado = ?1 WHERE id = ?2",
        [tiempo, id],
    )?;
    Ok(())
}

struct Timer {
    inicio: Instant,
    activo: bool,
}

struct TodoItem {
    id: i32,
    text: String,
    checked: bool,
    tiempo_acumulado: i32,
    temporizador: Option<Timer>,
}

impl TodoItem {
    fn tiempo_total(&self) -> i32 {
        if let Some(ref timer) = self.temporizador {
            if timer.activo {
                return self.tiempo_acumulado + timer.inicio.elapsed().as_secs() as i32;
            }
        }
        self.tiempo_acumulado
    }

    fn temporizador_activo(&self) -> bool {
        self.temporizador.as_ref().map_or(false, |t| t.activo)
    }

    fn pausar_temporizador(&mut self) {
        if let Some(ref timer) = self.temporizador {
            self.tiempo_acumulado += timer.inicio.elapsed().as_secs() as i32;
            let _ = actualizar_tiempo(self.id, self.tiempo_acumulado);
        }
        self.temporizador = None;
    }

    fn iniciar_temporizador(&mut self) {
        self.temporizador = Some(Timer {
            inicio: Instant::now(),
            activo: true,
        });
    }

    fn resetear_temporizador(&mut self) {
        self.tiempo_acumulado = 0;
        self.temporizador = None;
        let _ = actualizar_tiempo(self.id, 0);
    }
}

struct MyApp {
    todos: Vec<TodoItem>,
    nueva_tarea: String,
    drag_index: Option<usize>,
}

impl Default for MyApp {
    fn default() -> Self {
        let todos = cargar_tareas().unwrap_or_else(|_| Vec::new());
        Self {
            todos,
            nueva_tarea: String::new(),
            drag_index: None,
        }
    }
}

impl MyApp {
    fn todo_at(&self, idx: usize) -> &TodoItem {
        &self.todos[idx]
    }

    fn todo_at_mut(&mut self, idx: usize) -> &mut TodoItem {
        &mut self.todos[idx]
    }

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
                if agregar_tarea(&self.nueva_tarea).is_ok() {
                    self.nueva_tarea.clear();
                    self.reload_tasks();
                }
            }
        });

        ui.add_space(5.0);
        ui.separator();
        ui.add_space(5.0);
    }

    fn render_task_item(&mut self, ui: &mut egui::Ui, idx: usize) -> bool {
        let mut should_delete = false;
        
        let item_id = egui::Id::new("task").with(idx);
        let hover_id = egui::Id::new("task_hover").with(idx);
        let is_being_dragged = self.drag_index == Some(idx);
        
        // Frame con fondo para la tarea - colores para tema oscuro
        let frame = if is_being_dragged {
            egui::Frame::none()
                .fill(Color32::from_rgba_unmultiplied(70, 130, 180, 80))
                .stroke(Stroke::new(2.0, Color32::from_rgb(100, 200, 255)))
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
                // Ícono de arrastre más visible
                ui.vertical(|ui| {
                    ui.add_space(2.0);
                    let drag_label = egui::RichText::new("⣿").size(18.0).color(Color32::from_gray(150));
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
                
                // Checkbox con texto ajustable
                let todo = self.todo_at_mut(idx);
                let checked_before = todo.checked;
                
                ui.checkbox(&mut todo.checked, "");
                ui.label(&todo.text);
                
                if checked_before != todo.checked {
                    let _ = actualizar_tarea(todo.id, todo.checked);
                }
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    should_delete = self.render_task_controls(ui, idx);
                });
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

        if ui.button("🗑").clicked() {
            should_delete = true;
        }

        self.render_timer_display(ui, idx);
        self.render_timer_controls(ui, idx);

        if ui.button("🔄").clicked() {
            let todo = self.todo_at_mut(idx);
            todo.resetear_temporizador();
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
        let todo = self.todo_at_mut(idx);
        if todo.temporizador_activo() {
            if ui.button("⏸").clicked() {
                todo.pausar_temporizador();
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
                if let Some(rect) = ui.ctx().memory(|mem| mem.data.get_temp::<egui::Rect>(task_hover_id)) {
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

        ui.add_space(30.0);
    }

    fn reload_tasks(&mut self) {
        self.todos = cargar_tareas().unwrap_or_else(|_| Vec::new());
    }

    fn delete_task(&mut self, idx: usize) {
        let tarea_id = self.todos[idx].id;
        if eliminar_tarea(tarea_id).is_ok() {
            self.todos.remove(idx);
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint();
        
        // Calcular altura necesaria basada en número de tareas
        let num_tareas = self.todos.len();
        let task_height = 40.0; // Altura aproximada por tarea
        let header_height = 100.0; // Header + agregar tarea
        let stats_height = 150.0; // Estadísticas + botón recargar
        let needed_height = header_height + (num_tareas as f32 * task_height) + stats_height;
        
        // Ajustar tamaño de ventana automáticamente
        let min_height = 200.0;
        let max_height = 700.0;
        let target_height = needed_height.clamp(min_height, max_height);
        
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(500.0, target_height)));

        egui::CentralPanel::default()
            .frame(egui::Frame::none().inner_margin(egui::Margin::same(8.0)))
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(4.0, 2.0);
                
                self.render_header(ui);
                self.render_add_task(ui);
                
                // Usar ScrollArea solo si excede la altura máxima
                if needed_height > max_height {
                    egui::ScrollArea::vertical()
                        .max_height(max_height - header_height - stats_height)
                        .show(ui, |ui| {
                            self.render_tasks(ui);
                        });
                } else {
                    self.render_tasks(ui);
                }
                
                self.render_statistics(ui);
            });
    }
}
