use eframe::egui;
use rusqlite::{Connection, Result as SqlResult};
use std::time::Instant;
use rodio::{Decoder, OutputStream, Sink};
use std::fs::File;
use std::io::BufReader;
use std::sync::mpsc::{self, Sender};

fn main() -> Result<(), eframe::Error> {
    // Inicializar la base de datos
    init_database().expect("Error al inicializar la base de datos");
    
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([500.0, 400.0]),
        ..Default::default()
    };
    
    eframe::run_native(
        "Lista de Tareas - SQLite",
        options,
        Box::new(|_cc| Ok(Box::<MyApp>::default())),
    )
}

fn init_database() -> SqlResult<()> {
    let conn = Connection::open("tareas.db")?;
    
    // Crear tabla si no existe
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
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM tareas",
        [],
        |row| row.get(0),
    )?;
    
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
            conn.execute(
                "INSERT INTO tareas (descripcion) VALUES (?1)",
                [tarea],
            )?;
        }
    }
    
    Ok(())
}

fn cargar_tareas() -> SqlResult<Vec<TodoItem>> {
    let conn = Connection::open("tareas.db")?;
    let mut stmt = conn.prepare("SELECT id, descripcion, completada, tiempo_acumulado FROM tareas")?;
    
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
    stop_sender: Option<Sender<()>>,
}

struct TodoItem {
    id: i32,
    text: String,
    checked: bool,
    tiempo_acumulado: i32, // en segundos
    temporizador: Option<Timer>,
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

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Solicitar repintado continuo para actualizar los temporizadores
        ctx.request_repaint();
        
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("📋 Lista de Tareas - SQLite");
            
            ui.add_space(10.0);
            ui.separator();
            ui.add_space(10.0);
            
            // Sección para agregar nueva tarea
            ui.horizontal(|ui| {
                ui.label("Nueva tarea:");
                let text_edit = ui.text_edit_singleline(&mut self.nueva_tarea);
                
                // Detectar Enter para agregar tarea
                if text_edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    if !self.nueva_tarea.trim().is_empty() {
                        if agregar_tarea(&self.nueva_tarea).is_ok() {
                            self.nueva_tarea.clear();
                            self.todos = cargar_tareas().unwrap_or_else(|_| Vec::new());
                        }
                    }
                }
                
                if ui.button("➕ Agregar").clicked() {
                    if !self.nueva_tarea.trim().is_empty() {
                        if agregar_tarea(&self.nueva_tarea).is_ok() {
                            self.nueva_tarea.clear();
                            self.todos = cargar_tareas().unwrap_or_else(|_| Vec::new());
                        }
                    }
                }
            });
            
            ui.add_space(10.0);
            ui.separator();
            ui.add_space(10.0);
            
            // Mostrar cada tarea de la base de datos como checkbox con temporizador y botón eliminar
            let mut tarea_a_eliminar: Option<usize> = None;
            
            for (idx, todo) in self.todos.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    // Checkbox
                    let checked_before = todo.checked;
                    ui.checkbox(&mut todo.checked, &todo.text);
                    
                    // Si cambió el estado, actualizar en la base de datos
                    if checked_before != todo.checked {
                        let _ = actualizar_tarea(todo.id, todo.checked);
                    }
                    
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Botón eliminar
                        if ui.button("🗑").clicked() {
                            tarea_a_eliminar = Some(idx);
                        }
                        
                        // Temporizador
                        let tiempo_total = if let Some(ref timer) = todo.temporizador {
                            if timer.activo {
                                todo.tiempo_acumulado + timer.inicio.elapsed().as_secs() as i32
                            } else {
                                todo.tiempo_acumulado
                            }
                        } else {
                            todo.tiempo_acumulado
                        };
                        
                        let horas = tiempo_total / 3600;
                        let minutos = (tiempo_total % 3600) / 60;
                        let segundos = tiempo_total % 60;
                        
                        ui.label(format!("⏱ {:02}:{:02}:{:02}", horas, minutos, segundos));
                        
                        // Botón Start/Pause
                        let temporizador_activo = todo.temporizador.as_ref().map_or(false, |t| t.activo);
                        
                        if temporizador_activo {
                            if ui.button("⏸ Pausar").clicked() {
                                if let Some(ref timer) = todo.temporizador {
                                    // Acumular el tiempo transcurrido
                                    todo.tiempo_acumulado += timer.inicio.elapsed().as_secs() as i32;
                                    let _ = actualizar_tiempo(todo.id, todo.tiempo_acumulado);
                                    
                                    // Enviar señal para detener el audio
                                    if let Some(ref sender) = timer.stop_sender {
                                        let _ = sender.send(());
                                    }
                                }
                                todo.temporizador = None;
                            }
                        } else {
                            if ui.button("▶ Iniciar").clicked() {
                                // Crear canal para detener el audio
                                let (tx, rx) = mpsc::channel();
                                
                                // Reproducir sonido al iniciar en un hilo separado
                                std::thread::spawn(move || {
                                    if let Ok((_stream, stream_handle)) = OutputStream::try_default() {
                                        if let Ok(file) = File::open("data/sound.mp3") {
                                            let buf_reader = BufReader::new(file);
                                            if let Ok(source) = Decoder::new(buf_reader) {
                                                if let Ok(sink) = Sink::try_new(&stream_handle) {
                                                    sink.append(source);
                                                    
                                                    // Esperar señal de stop o que termine el audio
                                                    loop {
                                                        // Verificar si recibimos señal de stop
                                                        if rx.try_recv().is_ok() {
                                                            sink.stop();
                                                            break;
                                                        }
                                                        
                                                        // Verificar si el audio terminó
                                                        if sink.empty() {
                                                            break;
                                                        }
                                                        
                                                        std::thread::sleep(std::time::Duration::from_millis(100));
                                                    }
                                                }
                                            }
                                        }
                                    }
                                });
                                
                                todo.temporizador = Some(Timer {
                                    inicio: Instant::now(),
                                    activo: true,
                                    stop_sender: Some(tx),
                                });
                            }
                        }
                        
                        // Botón Reset
                        if ui.button("🔄").clicked() {
                            todo.tiempo_acumulado = 0;
                            todo.temporizador = None;
                            let _ = actualizar_tiempo(todo.id, 0);
                        }
                    });
                });
                ui.add_space(5.0);
            }
            
            // Eliminar tarea si se marcó para eliminación
            if let Some(idx) = tarea_a_eliminar {
                let tarea_id = self.todos[idx].id;
                if eliminar_tarea(tarea_id).is_ok() {
                    self.todos.remove(idx);
                }
            }
            
            ui.add_space(20.0);
            ui.separator();
            ui.add_space(10.0);
            
            // Calcular tiempo total de todas las tareas
            let tiempo_total_segundos: i32 = self.todos.iter().map(|t| {
                if let Some(ref timer) = t.temporizador {
                    if timer.activo {
                        t.tiempo_acumulado + timer.inicio.elapsed().as_secs() as i32
                    } else {
                        t.tiempo_acumulado
                    }
                } else {
                    t.tiempo_acumulado
                }
            }).sum();
            
            let horas_total = tiempo_total_segundos / 3600;
            let minutos_total = (tiempo_total_segundos % 3600) / 60;
            let segundos_total = tiempo_total_segundos % 60;
            
            // Estadísticas
            let total = self.todos.len();
            let completed = self.todos.iter().filter(|t| t.checked).count();
            let pending = total - completed;
            
            ui.label(format!("📊 Total: {}", total));
            ui.label(format!("✅ Completadas: {}", completed));
            ui.label(format!("⏳ Pendientes: {}", pending));
            ui.label(format!("⏱️ Tiempo total: {:02}:{:02}:{:02}", horas_total, minutos_total, segundos_total));
            
            ui.add_space(10.0);
            
            // Botón para recargar tareas
            if ui.button("🔄 Recargar tareas").clicked() {
                self.todos = cargar_tareas().unwrap_or_else(|_| Vec::new());
            }
        });
    }
}
