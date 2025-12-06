use eframe::egui::Ui;
use egui::{FontFamily, FontId, TextStyle};
use rusqlite::{Connection, Result as SqlResult};
use std::time::Instant;

const HEADING: &str = "📋 Lista de Tareas";

fn main() -> Result<(), eframe::Error> {
    init_database().expect("Error al inicializar la base de datos");

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([500.0, 500.0]),
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
}

impl Default for MyApp {
    fn default() -> Self {
        let todos = cargar_tareas().unwrap_or_else(|_| Vec::new());
        Self {
            todos,
            nueva_tarea: String::new(),
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
        ui.add_space(10.0);
        ui.separator();
        ui.add_space(10.0);
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

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(10.0);
    }

    fn render_task_item(&mut self, ui: &mut egui::Ui, idx: usize) -> bool {
        let mut should_delete = false;

        ui.horizontal(|ui| {
            self.render_task_checkbox(ui, idx);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                should_delete = self.render_task_controls(ui, idx);
            });
        });
        ui.add_space(5.0);

        should_delete
    }

    fn render_task_checkbox(&mut self, ui: &mut egui::Ui, idx: usize) {
        let todo = self.todo_at_mut(idx);
        let checked_before = todo.checked;
        ui.checkbox(&mut todo.checked, &todo.text);

        if checked_before != todo.checked {
            let _ = actualizar_tarea(todo.id, todo.checked);
        }
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

        for idx in 0..self.todos.len() {
            if self.render_task_item(ui, idx) {
                tarea_a_eliminar = Some(idx);
            }
        }

        if let Some(idx) = tarea_a_eliminar {
            self.delete_task(idx);
        }
    }

    fn render_statistics(&mut self, ui: &mut egui::Ui) {
        ui.add_space(20.0);
        ui.separator();
        ui.add_space(10.0);

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

        ui.add_space(10.0);

        if ui.button("🔄 Recargar tareas").clicked() {
            self.reload_tasks();
        }
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

        egui::CentralPanel::default().show(ctx, |ui| {
            self.render_header(ui);
            self.render_add_task(ui);
            self.render_tasks(ui);
            self.render_statistics(ui);
        });
    }
}
