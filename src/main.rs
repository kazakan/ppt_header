use yew::prelude::*;
use wasm_bindgen::prelude::*;
use web_sys::{HtmlInputElement, File, Event, EventTarget, DragEvent};
use gloo::file::callbacks::FileReader;
use gloo::file::File as GlooFile;
use rust_xlsxwriter::*;

mod pptx_parser;

#[derive(Clone, PartialEq, Debug)]
pub struct SlideData {
    pub page: usize,
    pub title: String,
    pub selected: bool,
}

pub enum Msg {
    FileSelected(Vec<File>),
    FileLoaded(String, Vec<u8>), // filename, data
    ParseFinished(Vec<SlideData>),
    ToggleSelect(usize), // index
    ToggleSelectAll,
    ExportCsv,
    ExportExcel,
    Error(String),
}

pub struct App {
    slides: Vec<SlideData>,
    file_name: Option<String>,
    is_loading: bool,
    error_msg: Option<String>,
    all_selected: bool,
    reader: Option<FileReader>,
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            slides: Vec::new(),
            file_name: None,
            is_loading: false,
            error_msg: None,
            all_selected: true,
            reader: None,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::FileSelected(files) => {
                if let Some(file) = files.first() {
                    let file = file.clone();
                    let file_name = file.name();
                    self.file_name = Some(file_name.clone());
                    self.is_loading = true;
                    self.error_msg = None;
                    self.slides.clear();

                    let link = ctx.link().clone();
                    // Using gloo to read file
                    let file = GlooFile::from(file);
                    let reader = gloo::file::callbacks::read_as_bytes(&file, move |res| {
                        match res {
                            Ok(data) => link.send_message(Msg::FileLoaded(file_name, data)),
                            Err(e) => link.send_message(Msg::Error(format!("Failed to read file: {}", e))),
                        }
                    });
                    self.reader = Some(reader);
                }
                true
            }
            Msg::FileLoaded(_name, data) => {
                // Here we will call the parser
                let link = ctx.link().clone();
                wasm_bindgen_futures::spawn_local(async move {
                   match pptx_parser::parse_pptx(&data).await {
                       Ok(slides) => link.send_message(Msg::ParseFinished(slides)),
                       Err(e) => link.send_message(Msg::Error(format!("Failed to parse PPTX: {}", e))),
                   }
                });
                true
            }
            Msg::ParseFinished(slides) => {
                self.slides = slides;
                self.is_loading = false;
                self.all_selected = true;
                true
            }
            Msg::ToggleSelect(idx) => {
                if let Some(slide) = self.slides.get_mut(idx) {
                    slide.selected = !slide.selected;
                }
                self.all_selected = self.slides.iter().all(|s| s.selected);
                true
            }
            Msg::ToggleSelectAll => {
                self.all_selected = !self.all_selected;
                for slide in &mut self.slides {
                    slide.selected = self.all_selected;
                }
                true
            }
            Msg::ExportCsv => {
                let content = self.generate_csv();
                download_file("slides.csv", content.as_bytes(), "text/csv;charset=utf-8");
                true
            }
            Msg::ExportExcel => {
                match self.generate_excel() {
                    Ok(content) => {
                         download_file("slides.xlsx", &content, "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet");
                    },
                    Err(e) => {
                        self.error_msg = Some(format!("Failed to generate Excel: {}", e));
                    }
                }
                true
            }
            Msg::Error(msg) => {
                self.error_msg = Some(msg);
                self.is_loading = false;
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="container mx-auto p-8 max-w-4xl">
                <header class="mb-8 text-center">
                    <h1 class="text-4xl font-bold text-gray-800 mb-2">{"PPTX Header Extractor"}</h1>
                    <p class="text-gray-600">{"Extract slide titles and export to CSV or Excel"}</p>
                </header>

                <div class="bg-white shadow-md rounded-lg p-6 mb-6">
                    <div class="flex items-center justify-center w-full">
                        <label for="dropzone-file" class="flex flex-col items-center justify-center w-full h-32 border-2 border-gray-300 border-dashed rounded-lg cursor-pointer bg-gray-50 hover:bg-gray-100"
                            ondragover={Callback::from(|e: DragEvent| {
                                e.prevent_default();
                            })}
                            ondragenter={Callback::from(|e: DragEvent| {
                                e.prevent_default();
                            })}
                            ondrop={ctx.link().callback(|e: DragEvent| {
                                e.prevent_default();
                                let mut files = Vec::new();
                                if let Some(data_transfer) = e.data_transfer() {
                                    if let Some(file_list) = data_transfer.files() {
                                        for i in 0..file_list.length() {
                                            if let Some(file) = file_list.item(i) {
                                                files.push(file);
                                            }
                                        }
                                    }
                                }
                                Msg::FileSelected(files)
                            })}
                        >
                            <div class="flex flex-col items-center justify-center pt-5 pb-6">
                                <p class="mb-2 text-sm text-gray-500"><span class="font-semibold">{"Click to upload"}</span>{" or drag and drop"}</p>
                                <p class="text-xs text-gray-500">{"PPTX files only"}</p>
                            </div>
                            <input id="dropzone-file" type="file" class="hidden" accept=".pptx" 
                                onchange={ctx.link().callback(|e: Event| {
                                    let input: HtmlInputElement = e.target().unwrap().dyn_into().unwrap();
                                    let mut files = Vec::new();
                                    if let Some(file_list) = input.files() {
                                        for i in 0..file_list.length() {
                                            if let Some(file) = file_list.item(i) {
                                                files.push(file);
                                            }
                                        }
                                    }
                                    Msg::FileSelected(files)
                                })}
                            />
                        </label>
                    </div>
                    if let Some(name) = &self.file_name {
                        <p class="mt-2 text-sm text-gray-600 text-center">{"Selected file: "} <span class="font-medium">{name}</span></p>
                    }
                    if self.is_loading {
                        <p class="mt-2 text-blue-600 text-center">{"Processing..."}</p>
                    }
                    if let Some(err) = &self.error_msg {
                         <p class="mt-2 text-red-600 text-center">{err}</p>
                    }
                </div>

                if !self.slides.is_empty() {
                    <div class="bg-white shadow-md rounded-lg overflow-hidden">
                        <div class="p-4 border-b flex justify-between items-center bg-gray-50">
                            <h2 class="text-lg font-semibold text-gray-700">{"Extracted Slides"}</h2>
                            <div class="flex gap-2">
                                <button onclick={ctx.link().callback(|_| Msg::ExportCsv)} class="bg-green-600 hover:bg-green-700 text-white font-bold py-2 px-4 rounded transition duration-150">
                                    {"Export CSV"}
                                </button>
                                <button onclick={ctx.link().callback(|_| Msg::ExportExcel)} class="bg-blue-600 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded transition duration-150">
                                    {"Export Excel"}
                                </button>
                            </div>
                        </div>
                        <div class="overflow-x-auto">
                            <table class="w-full text-sm text-left text-gray-500">
                                <thead class="text-xs text-gray-700 uppercase bg-gray-100">
                                    <tr>
                                        <th scope="col" class="p-4">
                                            <div class="flex items-center">
                                                <input type="checkbox" checked={self.all_selected} 
                                                    onclick={ctx.link().callback(|_| Msg::ToggleSelectAll)}
                                                    class="w-4 h-4 text-blue-600 bg-gray-100 border-gray-300 rounded focus:ring-blue-500" />
                                            </div>
                                        </th>
                                        <th scope="col" class="px-6 py-3">{"Page"}</th>
                                        <th scope="col" class="px-6 py-3">{"Slide Title"}</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    { for self.slides.iter().enumerate().map(|(idx, slide)| {
                                        html! {
                                            <tr class="bg-white border-b hover:bg-gray-50">
                                                <td class="w-4 p-4">
                                                    <div class="flex items-center">
                                                        <input type="checkbox" checked={slide.selected}
                                                            onclick={ctx.link().callback(move |_| Msg::ToggleSelect(idx))}
                                                            class="w-4 h-4 text-blue-600 bg-gray-100 border-gray-300 rounded focus:ring-blue-500" />
                                                    </div>
                                                </td>
                                                <td class="px-6 py-4 font-medium text-gray-900 whitespace-nowrap">{slide.page}</td>
                                                <td class="px-6 py-4">{&slide.title}</td>
                                            </tr>
                                        }
                                    }) }
                                </tbody>
                            </table>
                        </div>
                    </div>
                }
            </div>
        }
    }
}

impl App {
    fn generate_csv(&self) -> String {
        let mut csv = String::from("\u{FEFF}Page,SlideTitle\n");
        for slide in &self.slides {
            if slide.selected {
                let title = slide.title.replace("\"", "\"\"");
                csv.push_str(&format!("{},\"{}\"\n", slide.page, title));
            }
        }
        csv
    }

    fn generate_excel(&self) -> Result<Vec<u8>, XlsxError> {
        let mut workbook = Workbook::new();
        let worksheet = workbook.add_worksheet();

        // Write header
        worksheet.write_string(0, 0, "Page")?;
        worksheet.write_string(0, 1, "SlideTitle")?;

        let mut row = 1;
        for slide in &self.slides {
            if slide.selected {
                worksheet.write_number(row, 0, slide.page as f64)?;
                worksheet.write_string(row, 1, &slide.title)?;
                row += 1;
            }
        }

        workbook.save_to_buffer()
    }
}

fn download_file(filename: &str, content: &[u8], mime_type: &str) {
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    // Create a Uint8Array from slice to pass to Blob
    let array = js_sys::Uint8Array::from(content);
    let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(
        &js_sys::Array::of1(&array), 
        web_sys::BlobPropertyBag::new().type_(mime_type)
    ).unwrap();
    
    let url = web_sys::Url::create_object_url_with_blob(&blob).unwrap();
    let a = document.create_element("a").unwrap().dyn_into::<web_sys::HtmlAnchorElement>().unwrap();
    a.set_href(&url);
    a.set_download(filename);
    a.click();
    web_sys::Url::revoke_object_url(&url).unwrap();
}

fn main() {
    yew::Renderer::<App>::new().render();
}
