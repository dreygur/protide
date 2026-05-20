use gpui::Context;
use super::*;
use super::super::request_types::KvList;

impl<E: WebSocketExecutor> RequestPanel<E> {
    pub(super) fn toggle_header(&mut self, index: usize, cx: &mut Context<Self>) {
        if let Some(header) = self.headers.get_mut(index) {
            header.enabled = !header.enabled;
            cx.notify();
        }
    }

    #[allow(dead_code)]
    pub(super) fn add_header(&mut self, cx: &mut Context<Self>) {
        self.headers.push(KeyValuePair::default());
        cx.notify();
    }

    pub(super) fn remove_header(&mut self, index: usize, cx: &mut Context<Self>) {
        if index < self.headers.len() && self.headers.len() > 1 {
            self.headers.remove(index);
            if let Some(target) = self.active_edit {
                match target {
                    EditTarget::HeaderKey(i) | EditTarget::HeaderValue(i) if i == index => {
                        self.active_edit = None;
                    }
                    EditTarget::HeaderKey(i) if i > index => {
                        self.active_edit = Some(EditTarget::HeaderKey(i - 1));
                    }
                    EditTarget::HeaderValue(i) if i > index => {
                        self.active_edit = Some(EditTarget::HeaderValue(i - 1));
                    }
                    _ => {}
                }
            }
            cx.notify();
        }
    }

    pub(super) fn toggle_grpc_meta(&mut self, index: usize, cx: &mut Context<Self>) {
        if let Some(meta) = self.grpc_metadata.get_mut(index) {
            meta.enabled = !meta.enabled;
            cx.notify();
        }
    }

    pub(super) fn add_grpc_meta(&mut self, cx: &mut Context<Self>) {
        self.grpc_metadata.push(KeyValuePair::default());
        cx.notify();
    }

    pub(super) fn remove_grpc_meta(&mut self, index: usize, cx: &mut Context<Self>) {
        if index < self.grpc_metadata.len() && self.grpc_metadata.len() > 1 {
            self.grpc_metadata.remove(index);
            if let Some(target) = self.active_edit {
                match target {
                    EditTarget::GrpcMetaKey(i) | EditTarget::GrpcMetaValue(i) if i == index => {
                        self.active_edit = None;
                    }
                    EditTarget::GrpcMetaKey(i) if i > index => {
                        self.active_edit = Some(EditTarget::GrpcMetaKey(i - 1));
                    }
                    EditTarget::GrpcMetaValue(i) if i > index => {
                        self.active_edit = Some(EditTarget::GrpcMetaValue(i - 1));
                    }
                    _ => {}
                }
            }
            cx.notify();
        }
    }

    pub(super) fn toggle_param(&mut self, index: usize, cx: &mut Context<Self>) {
        if let Some(param) = self.params.get_mut(index) {
            param.enabled = !param.enabled;
            self.sync_url_from_params(cx);
            cx.notify();
        }
    }

    #[allow(dead_code)]
    pub(super) fn add_param(&mut self, cx: &mut Context<Self>) {
        self.params.push(KeyValuePair::default());
        cx.notify();
    }

    pub(super) fn remove_param(&mut self, index: usize, cx: &mut Context<Self>) {
        if index < self.params.len() && self.params.len() > 1 {
            self.params.remove(index);
            if let Some(target) = self.active_edit {
                match target {
                    EditTarget::ParamKey(i) | EditTarget::ParamValue(i) if i == index => {
                        self.active_edit = None;
                    }
                    EditTarget::ParamKey(i) if i > index => {
                        self.active_edit = Some(EditTarget::ParamKey(i - 1));
                    }
                    EditTarget::ParamValue(i) if i > index => {
                        self.active_edit = Some(EditTarget::ParamValue(i - 1));
                    }
                    _ => {}
                }
            }
            self.sync_url_from_params(cx);
            cx.notify();
        }
    }

    pub(super) fn toggle_form_field(&mut self, index: usize, cx: &mut Context<Self>) {
        if let Some(field) = self.form_data.get_mut(index) {
            field.enabled = !field.enabled;
            cx.notify();
        }
    }

    pub(super) fn add_form_field(&mut self, cx: &mut Context<Self>) {
        self.form_data.push(FormField::default());
        cx.notify();
    }

    pub(super) fn toggle_form_field_type(&mut self, index: usize, cx: &mut Context<Self>) {
        if let Some(field) = self.form_data.get_mut(index) {
            field.field_type = match field.field_type {
                FormFieldType::Text => FormFieldType::File,
                FormFieldType::File => FormFieldType::Text,
            };
            if field.field_type == FormFieldType::Text {
                field.file_path = None;
                field.value.clear();
            }
            cx.notify();
        }
    }

    pub(super) fn select_form_file(&mut self, index: usize, cx: &mut Context<Self>) {
        let mut dialog = rfd::FileDialog::new();
        if let Some(dir) = last_paths::last_dir("form_file") {
            dialog = dialog.set_directory(dir);
        }
        if let Some(path) = dialog.pick_file() {
            last_paths::save_last_dir("form_file", &path);
            if let Some(field) = self.form_data.get_mut(index) {
                field.value = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("file")
                    .to_string();
                field.file_path = Some(path);
                cx.notify();
            }
        }
    }

    pub(super) fn remove_form_field(&mut self, index: usize, cx: &mut Context<Self>) {
        if index < self.form_data.len() && self.form_data.len() > 1 {
            self.form_data.remove(index);
            if let Some(target) = self.active_edit {
                match target {
                    EditTarget::FormKey(i) | EditTarget::FormValue(i) if i == index => {
                        self.active_edit = None;
                    }
                    EditTarget::FormKey(i) if i > index => {
                        self.active_edit = Some(EditTarget::FormKey(i - 1));
                    }
                    EditTarget::FormValue(i) if i > index => {
                        self.active_edit = Some(EditTarget::FormValue(i - 1));
                    }
                    _ => {}
                }
            }
            cx.notify();
        }
    }

    pub(super) fn reorder_kv(&mut self, list: KvList, from: usize, to: usize, cx: &mut Context<Self>) {
        let vec = match list {
            KvList::Params => &mut self.params,
            KvList::Headers => &mut self.headers,
            KvList::GrpcMeta => &mut self.grpc_metadata,
        };
        if from < vec.len() && to < vec.len() {
            let item = vec.remove(from);
            vec.insert(to.min(vec.len()), item);
        }
        cx.notify();
    }

    pub(super) fn reorder_form_field(&mut self, from: usize, to: usize, cx: &mut Context<Self>) {
        if from < self.form_data.len() && to < self.form_data.len() {
            let item = self.form_data.remove(from);
            self.form_data.insert(to.min(self.form_data.len()), item);
        }
        cx.notify();
    }
}
