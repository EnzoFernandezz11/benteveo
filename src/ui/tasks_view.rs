use crate::domain::task::TaskList;

pub struct TasksResponse {
    pub changed: bool,
}

pub fn show(ui: &mut egui::Ui, tasks: &mut TaskList, draft: &mut String) -> TasksResponse {
    let mut changed = false;
    ui.heading("// tareas");
    ui.horizontal(|ui| {
        let response = ui.add(
            egui::TextEdit::singleline(draft)
                .hint_text("Nueva tarea…")
                .desired_width(420.0),
        );
        let submit = ui.button("agregar").clicked()
            || (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));
        if submit && !draft.trim().is_empty() {
            tasks.add(draft.trim());
            draft.clear();
            changed = true;
        }
    });
    ui.add_space(8.0);
    if ui.button("limpiar completadas").clicked() {
        changed |= tasks.clear_completed() > 0;
    }
    egui::ScrollArea::vertical().show(ui, |ui| {
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
                let mut edited = text.clone();
                if ui
                    .add(egui::TextEdit::singleline(&mut edited).desired_width(320.0))
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
        }
    });
    TasksResponse { changed }
}
