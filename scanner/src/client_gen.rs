use std::ascii::AsciiExt;
use std::borrow::Cow;
use std::io::Write;

use protocol::*;

pub fn generate_client_api<O: Write>(protocol: Protocol, out: &mut O) {

    writeln!(out, "//\n// This file was auto-generated, do not edit directly\n//\n").unwrap();

    if let Some(text) = protocol.copyright {
        writeln!(out, "/*\n{}\n*/\n", text).unwrap();
    }

    writeln!(out, "use wayland_sys::common::*;").unwrap();
    writeln!(out, "use wayland_sys::client::*;").unwrap();
    writeln!(out, "use {{Proxy, ProxyId, wrap_proxy}};").unwrap();
    writeln!(out, "use events::{{EventIterator, EventFifo, get_eventiter_internals, eventiter_from_internals}};").unwrap();
    writeln!(out, "// update if needed to the appropriate file\nuse super::interfaces::*;\n").unwrap();
    writeln!(out, "use std::ffi::{{CString, CStr}};").unwrap();
    writeln!(out, "use std::ptr;").unwrap();
    writeln!(out, "use std::sync::Arc;").unwrap();
    writeln!(out, "use std::sync::atomic::{{AtomicBool, Ordering}};").unwrap();
    writeln!(out, "use libc::{{c_void, c_char}};").unwrap();

    // envent handling

    writeln!(out, "/// An event generated by the protocol {}.", protocol.name).unwrap();
    writeln!(out, "///").unwrap();
    writeln!(out, "/// Each variant is composed of a `ProxyId` reffering to the proxy object").unwrap();
    writeln!(out, "/// and of the event data itself.").unwrap();
    writeln!(out, "#[derive(Debug)]").unwrap();
    writeln!(out, "pub enum {}ProtocolEvent {{", snake_to_camel(&protocol.name)).unwrap();
    for interface in &protocol.interfaces {
        if interface.events.len() > 0 {
            writeln!(out, "    {}(ProxyId, {}Event),",
                snake_to_camel(&interface.name), snake_to_camel(&interface.name)).unwrap();
        }
    }
    writeln!(out, "}}\n").unwrap();

    writeln!(out, "type {}_dispatcher_implem = fn(*mut wl_proxy, u32, *const wl_argument) -> Option<{}ProtocolEvent>;\n",
        protocol.name, snake_to_camel(&protocol.name)).unwrap();

    writeln!(out,
        "extern \"C\" fn event_dispatcher(implem: *const c_void, proxy: *mut c_void, opcode: u32, _: *const wl_message, args: *const wl_argument) {{").unwrap();
    writeln!(out, "    let userdata = unsafe {{ ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_get_user_data, proxy as *mut wl_proxy) }} as *const (EventFifo, AtomicBool);").unwrap();
    writeln!(out, "    if userdata.is_null() {{ return; }}").unwrap();
    writeln!(out, "    let fifo: &(EventFifo, AtomicBool) = unsafe {{ &*userdata }};").unwrap();
    writeln!(out, "    if !fifo.1.load(Ordering::SeqCst) {{ return; }}").unwrap();
    writeln!(out, "    let implem = unsafe {{ ::std::mem::transmute::<_, {}_dispatcher_implem>(implem) }};",
        protocol.name).unwrap();
    writeln!(out, "    let event = implem(proxy as *mut wl_proxy, opcode, args);").unwrap();
    writeln!(out, "    if let Some(evt) = event {{").unwrap();
    writeln!(out, "        fifo.0.push(::Event::{}(evt));", snake_to_camel(&protocol.name)).unwrap();
    writeln!(out, "    }}").unwrap();
    writeln!(out, "}}").unwrap();

    for interface in protocol.interfaces {
        let camel_iname = snake_to_camel(&interface.name);
        writeln!(out, "//\n// interface {}\n//\n", interface.name).unwrap();

        if let Some((ref summary, ref desc)) = interface.description {
            write_doc(summary, desc, "", out)
        }
        writeln!(out, "pub struct {} {{\n    ptr: *mut wl_proxy,\n    evq: Arc<(EventFifo,AtomicBool)>\n}}\n",
            camel_iname).unwrap();

        writeln!(out, "unsafe impl Sync for {} {{}}", camel_iname).unwrap();
        writeln!(out, "unsafe impl Send for {} {{}}", camel_iname).unwrap();

        writeln!(out, "impl Proxy for {} {{", camel_iname).unwrap();
        writeln!(out, "    fn ptr(&self) -> *mut wl_proxy {{ self.ptr }}").unwrap();
        writeln!(out, "    fn interface() -> *mut wl_interface {{ unsafe {{ &mut {}_interface  as *mut wl_interface }} }}", interface.name).unwrap();
        writeln!(out, "    fn interface_name() -> &'static str {{ \"{}\" }}", interface.name).unwrap();
        writeln!(out, "    fn version() -> u32 {{ {} }}", interface.version).unwrap();
        writeln!(out, "    fn id(&self) -> ProxyId {{ ProxyId {{ id: self.ptr as usize }} }}").unwrap();
        writeln!(out, "    unsafe fn from_ptr(ptr: *mut wl_proxy) -> {} {{", camel_iname).unwrap();
        if interface.name != "wl_display" && interface.events.len() > 0 {
            writeln!(out, "        ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_add_dispatcher, ptr, event_dispatcher, {}_implem as *const c_void, ptr::null_mut());", interface.name).unwrap();
        }
        writeln!(out, "        {} {{ ptr: ptr, evq: Arc::new((EventFifo::new(), AtomicBool::new(false))) }}", camel_iname).unwrap();
        writeln!(out, "    }}").unwrap();
        writeln!(out, "    fn set_evt_iterator(&mut self, evt: &EventIterator) {{").unwrap();
        writeln!(out, "        self.evq = get_eventiter_internals(evt);").unwrap();
        if interface.name != "wl_display" {
        writeln!(out, "        let ptr = &*self.evq as *const (EventFifo,AtomicBool);").unwrap();
        writeln!(out, "        unsafe {{ ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_set_user_data, self.ptr, ptr as *const c_void as *mut c_void) }};").unwrap();
        }
        writeln!(out, "    }}").unwrap();
        writeln!(out, "}}\n").unwrap();

        writeln!(out, "impl ::std::fmt::Debug for {} {{", camel_iname).unwrap();
        writeln!(out, "    fn fmt(&self, fmt: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error> {{").unwrap();
        writeln!(out, "        fmt.write_fmt(format_args!(\"{}::{}::{{}}\", self.ptr as usize))", protocol.name, interface.name).unwrap();
        writeln!(out, "    }}").unwrap();
        writeln!(out, "}}\n").unwrap();
            

        // emit enums
        for enu in interface.enums {
            if let Some((ref summary, ref desc)) = enu.description {
                write_doc(summary, desc, "", out)
            }
            writeln!(out, "#[repr(i32)]\npub enum {}{} {{",
                snake_to_camel(&interface.name), snake_to_camel(&enu.name)).unwrap();
            if enu.entries.len() == 1 {
                writeln!(out, "    #[doc(hidden)]").unwrap();
                writeln!(out, "    __not_univariant = -1,").unwrap();
            }
            for entry in enu.entries {
                if let Some(summary) = entry.summary {
                    writeln!(out, "    /// {}", summary).unwrap();
                }
                let variantname = snake_to_camel(&entry.name);
                if variantname.chars().next().unwrap().is_digit(10) {
                    writeln!(out, "    {}{} = {},",
                        enu.name.chars().next().unwrap().to_ascii_uppercase(),
                        variantname, entry.value).unwrap();
                } else {
                    writeln!(out, "    {} = {},", variantname, entry.value).unwrap();
                }
            }
            writeln!(out, "}}\n").unwrap();
        }

        // emit opcodes
        writeln!(out, "// {} opcodes", interface.name).unwrap();
        let mut i = 0;
        for req in &interface.requests {
            writeln!(out, "const {}_{}: u32 = {};",
                snake_to_screaming(&interface.name), snake_to_screaming(&req.name), i).unwrap();
            i += 1;
        }
        if i > 0 { writeln!(out, "").unwrap() }

        // emit events
        if interface.events.len() > 0 {
            writeln!(out, "#[derive(Debug)]").unwrap();
            writeln!(out, "pub enum {}Event {{", camel_iname).unwrap();
            for evt in &interface.events {
                if let Some((ref summary, ref desc)) = evt.description {
                    write_doc(summary, desc, "    ", out)
                }
                if evt.args.len() > 0 {
                    write!(out, "    ///\n    /// Values:").unwrap();
                    for a in &evt.args {
                        write!(out, " {},", a.name).unwrap();
                    }
                    writeln!(out, "").unwrap();
                }
                write!(out, "    {}", snake_to_camel(&evt.name)).unwrap();
                if evt.args.len() > 0 {
                    write!(out, "(").unwrap();
                    for a in &evt.args {
                        if a.typ == Type::NewId {
                            write!(out, "{},",
                                a.interface.as_ref().map(|s| snake_to_camel(s))
                                    .expect("Cannot create a new_id in an event without an interface.")
                            ).unwrap();
                        } else {
                            write!(out, "{},", a.typ.rust_type()).unwrap();
                        }
                    }
                    write!(out, ")").unwrap();
                }
                writeln!(out, ",").unwrap();
            }
            writeln!(out, "}}\n").unwrap();

            // event handler
            writeln!(out, "fn {}_implem(proxy: *mut wl_proxy, opcode: u32, args: *const wl_argument) -> Option<{}ProtocolEvent> {{",
                interface.name, snake_to_camel(&protocol.name)).unwrap();
            writeln!(out, "    let event = match opcode {{").unwrap();
            for (op, evt) in interface.events.iter().enumerate() {
                writeln!(out, "        {} => {{", op).unwrap();
                for (i, arg) in evt.args.iter().enumerate() {
                    write!(out, "            let arg_{} = unsafe {{", i).unwrap();
                    match arg.typ {
                        Type::Uint => write!(out, "*(args.offset({}) as *const u32)", i),
                        Type::Int | Type::Fd => write!(out, "*(args.offset({}) as *const i32)", i),
                        Type::Fixed => write!(out, "wl_fixed_to_double(*(args.offset({}) as *const i32))", i),
                        Type::Object => write!(out, "wrap_proxy(*(args.offset({}) as *const *mut wl_proxy))", i),
                        Type::String => write!(out, "String::from_utf8_lossy(CStr::from_ptr(*(args.offset({}) as *const *mut c_char)).to_bytes()).into_owned()", i),
                        Type::Array => write!(out, "{{ let array = *(args.offset({}) as *const *mut wl_array); ::std::slice::from_raw_parts((*array).data as *const u8, (*array).size as usize).to_owned() }}", i),
                        Type::NewId => write!(out, "{{ let ptr = *(args.offset({}) as *const *mut wl_proxy); {}::from_ptr(ptr) }}", i, snake_to_camel(arg.interface.as_ref().unwrap())),
                        Type::Destructor => unreachable!()
                    }.unwrap();
                    writeln!(out, "}};").unwrap();
                }
                write!(out, "            Some({}Event::{}", camel_iname, snake_to_camel(&evt.name)).unwrap();
                if evt.args.len() > 0 {
                    write!(out, "(").unwrap();
                    for i in 0..evt.args.len() {
                        write!(out, "arg_{},", i).unwrap();
                    }
                    write!(out, ")").unwrap();
                }
                writeln!(out, ")").unwrap();
                writeln!(out, "        }},").unwrap();
            }

            writeln!(out, "        _ => None").unwrap();
            writeln!(out, "    }};").unwrap();
            writeln!(out, "    event.map(|event| {}ProtocolEvent::{}(wrap_proxy(proxy), event))", snake_to_camel(&protocol.name), camel_iname).unwrap();
            writeln!(out, "}}\n").unwrap();
        }

        // impl
        writeln!(out, "impl {} {{", camel_iname).unwrap();
        // requests
        for req in &interface.requests {
            let new_id_interfaces: Vec<Option<String>> = req.args.iter()
                .filter(|a| a.typ == Type::NewId)
                .map(|a| a.interface.clone())
                .collect();
            if new_id_interfaces.len() > 1 {
                // TODO: can we handle this properly ?
                continue;
            }
            let ret = new_id_interfaces.into_iter().next();

            writeln!(out, "").unwrap();

            if let Some((ref summary, ref doc)) = req.description {
                write_doc(summary, doc, "    ", out);
            }
            if req.since > 1 {
                writeln!(out, "    ///\n    /// Requires interface version `>= {}`.", req.since).unwrap();
            }
            write!(out, "    pub ").unwrap();
            if let Some(ref newint) = ret {
                if newint.is_none() {
                    write!(out, "unsafe ").unwrap();
                }
            }
            write!(out, "fn {}{}", req.name, if ::util::is_keyword(&req.name) { "_" } else { "" }).unwrap();
            if let Some(ref newint) = ret {
                if newint.is_none() {
                    write!(out, "<T: Proxy>").unwrap();
                }
            }
            if req.typ == Some(Type::Destructor) {
                write!(out, "(self,").unwrap();
            } else {
                write!(out, "(&self,").unwrap();
            }
            for a in &req.args {
                if a.typ == Type::NewId { continue; }
                let typ: Cow<str> = if a.typ == Type::Object {
                    a.interface.as_ref().map(|i| format!("&{}", snake_to_camel(i)).into()).unwrap_or("*mut ()".into())
                } else {
                    a.typ.rust_type().into()
                };
                if a.allow_null {
                    write!(out, " {}: Option<{}>,", a.name, typ).unwrap();
                } else {
                    write!(out, " {}: {},", a.name, typ).unwrap();
                }
            }
            if let Some(ref newint) = ret {
                if newint.is_none() {
                    write!(out, "version: u32,").unwrap();
                }
            }
            write!(out, ")").unwrap();
            if let Some(ref newint) = ret {
                write!(out, " -> {}", newint.as_ref().map(|t| snake_to_camel(t)).unwrap_or("T".to_owned())).unwrap();
            }
            writeln!(out, " {{").unwrap();
            // function body
            if let Some(ref newint) = ret {
                if newint.is_none() {
                    writeln!(out, "        if version < <T as Proxy>::version() {{").unwrap();
                    writeln!(out, "            panic!(\"Tried to bind interface {{}} with version {{}} while it is only supported up to {{}}.\", <T as Proxy>::interface_name(), version, <T as Proxy>::version())").unwrap();
                    writeln!(out, "        }}").unwrap();
                }
            }
            for a in &req.args {
                match a.typ {
                    Type::String => {
                        if a.allow_null {
                            writeln!(out, "        let {} = {}.map(|t| CString::new(t).unwrap_or_else(|_| panic!(\"Got a String with interior null.\")));",
                                a.name, a.name).unwrap();
                        } else {
                            writeln!(out, "        let {} = CString::new({}).unwrap_or_else(|_| panic!(\"Got a String with interior null.\"));",
                                a.name, a.name).unwrap();
                        }
                    },
                    Type::Fixed => {
                        writeln!(out, "        let {} = wl_fixed_from_double({})", a.name, a.name).unwrap();
                    },
                    _ => {}
                }
            }
            if let Some(ref newint) = ret {
                if let &Some(ref name) = newint {
                    writeln!(out, "        let ptr = unsafe {{ ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_marshal_constructor, self.ptr(), {}_{}, &{}_interface as *const wl_interface",
                        snake_to_screaming(&interface.name), snake_to_screaming(&req.name), name).unwrap();
                } else {
                    writeln!(out, "        let ptr = unsafe {{ ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_marshal_constructor, self.ptr(), {}_{}, <T as Proxy>::interface()",
                        snake_to_screaming(&interface.name), snake_to_screaming(&req.name)).unwrap();
                }
            } else {
                writeln!(out, "        unsafe {{ ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_marshal, self.ptr(), {}_{}",
                    snake_to_screaming(&interface.name), snake_to_screaming(&req.name)).unwrap();
            }
            write!(out, "           ").unwrap();
            for a in &req.args {
                if a.typ == Type::NewId {
                    if let Some(ref newint) = ret {
                        if newint.is_none() {
                            write!(out, ", (*<T as Proxy>::interface()).name, version").unwrap();
                        }
                    }
                    write!(out, ", ptr::null_mut::<wl_proxy>()").unwrap();
                } else if a.typ == Type::String {
                    if a.allow_null {
                        write!(out, ", {}.map(|s| s.as_ptr()).unwrap_or(ptr::null())", a.name).unwrap();
                    } else {
                        write!(out, ", {}.as_ptr()", a.name).unwrap();
                    }
                } else if a.typ == Type::Array {
                    if a.allow_null {
                        write!(out, ", {}.map(|a| &mut a as *mut wl_array).unwrap_or(ptr::null_mut())", a.name).unwrap();
                    } else {
                        write!(out, ", &mut {} as *mut wl_array", a.name).unwrap();
                    }
                } else if a.typ == Type::Object {
                    if a.allow_null {
                        write!(out, ", {}.map(Proxy::ptr).unwrap_or(ptr::null_mut())", a.name).unwrap();
                    } else {
                        write!(out, ", {}.ptr()", a.name).unwrap();
                    }
                } else {
                    if a.allow_null {
                        write!(out, ", {}.unwrap_or(0)", a.name).unwrap();
                    } else {
                        write!(out, ", {}", a.name).unwrap();
                    }
                }
            }
            writeln!(out, ") }};").unwrap();
            if req.typ == Some(Type::Destructor) {
                writeln!(out, "        unsafe {{ ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_destroy, self.ptr()) }}").unwrap();
            }
            if let Some(ref newint) = ret {
                writeln!(out, "        let mut proxy: {} = unsafe {{ Proxy::from_ptr(ptr) }};",
                    newint.as_ref().map(|t| snake_to_camel(t)).unwrap_or("T".to_owned())).unwrap();
                writeln!(out, "        let evt_iter = eventiter_from_internals(self.evq.clone());").unwrap();
                writeln!(out, "        proxy.set_evt_iterator(&evt_iter);").unwrap();
                writeln!(out, "        ::std::mem::forget(evt_iter); // Don't run the destructor !").unwrap();
                writeln!(out, "        proxy").unwrap();
            }
            writeln!(out, "    }}").unwrap();
        }

        writeln!(out, "}}\n").unwrap();

        if interface.name != "wl_display" {
            writeln!(out, "impl Drop for {} {{", camel_iname).unwrap();
            writeln!(out, "    fn drop(&mut self) {{").unwrap();
            writeln!(out, "        unsafe {{ ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_destroy, self.ptr()) }}").unwrap();
            writeln!(out, "    }}").unwrap();
            writeln!(out, "}}").unwrap();
        }
    }
    
}

fn write_doc<O: Write>(summary: &str, contents: &str, prefix: &str, out: &mut O) {
    writeln!(out, "{}/// {}", prefix, summary).unwrap();
    writeln!(out, "{}///", prefix).unwrap();
    for l in contents.lines() {
        let trimmed = l.trim();
        if trimmed.len() > 0 {
            writeln!(out, "{}/// {}", prefix, trimmed).unwrap();
        } else {
            writeln!(out, "{}///", prefix).unwrap();
        }
    }
}

fn snake_to_camel(input: &str) -> String {
    input.split('_').flat_map(|s| {
        let mut first = true;
        s.chars().map(move |c| {
            if first {
                first = false;
                c.to_ascii_uppercase()
            } else {
                c
            }
        })
    }).collect()
}

fn snake_to_screaming(input: &str) -> String {
    input.chars().map(|c| c.to_ascii_uppercase()).collect()
}
