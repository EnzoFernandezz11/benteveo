use crate::domain::task::TaskList;
use crate::ui::theme;

pub struct TasksResponse {
    pub changed: bool,
}

pub fn show(ui: &mut egui::Ui, tasks: &mut TaskList, draft: &mut String) -> TasksResponse {
    let mut changed = false;
    theme::terminal_panel(ui, |ui| {
        theme::title(ui, "03", "TASK QUEUE");
        theme::label(ui, format!("{} ITEMS EN LA COLA", tasks.len()));
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            let response = ui.add(
                egui::TextEdit::singleline(draft)
                    .hint_text("+ nueva tarea")
                    .desired_width(365.0),
            );
            let submit = ui
                .add_sized([108.0, 32.0], egui::Button::new("AGREGAR"))
                .clicked()
                || (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));
            if submit && !draft.trim().is_empty() {
                tasks.add(draft.trim());
                draft.clear();
                changed = true;
            }
        });
        ui.horizontal(|ui| {
            if ui.button("LIMPIAR COMPLETADAS").clicked() {
                changed |= tasks.clear_completed() > 0;
            }
            theme::label(ui, "ENTER PARA AGREGAR");
        });
        ui.add_space(6.0);
        ui.separator();
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                let rows: Vec<_> = tasks
                    .iter()
                    .map(|task| (task.id, task.text.clone(), task.completed, task.position))
                    .collect();
                for (id, text, completed, position) in rows {
                    ui.horizontal(|ui| {
                        let mut done = completed;
                        if ui.checkbox(&mut done, "").changed() {
                            tasks.toggle(id);
                            changed = true;
                        }
                        ui.label(
                            egui::RichText::new(format!("{:02}", position + 1))
                                .color(theme::ACCENT),
                        );
                        let mut edited = text;
                        if ui
                            .add(egui::TextEdit::singleline(&mut edited).desired_width(290.0))
                            .changed()
                            && !edited.trim().is_empty()
                        {
                            tasks.edit(id, edited);
                            changed = true;
                        }
                        if ui.small_button("↑").clicked() && position > 0 {
                            tasks.reorder(id, position - 1);
                            changed = true;
                        }
                        if ui.small_button("↓").clicked() {
                            tasks.reorder(id, position + 1);
                            changed = true;
                        }
                        if ui.small_button("×").clicked() {
                            tasks.delete(id);
                            changed = true;
                        }
                    });
                    ui.separator();
                }
            });
    });
    TasksResponse { changed }
}
