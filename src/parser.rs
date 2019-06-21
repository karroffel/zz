use pest::Parser;
use super::ast::*;
use std::collections::HashMap;
use std::path::Path;
use std::io::{Read};

#[derive(Parser)]
#[grammar = "zz.pest"]
pub struct ZZParser;

pub fn parse(modules: &mut HashMap<String, Module>, n: &Path)
{
    match p(modules, &n){
        Err(e) => {
            eprintln!("{:?} : {}", n, e);
            std::process::exit(9);
        }
        Ok(md) => {
            modules.insert(md.name.clone(), md);
        }
    }
}

fn p(modules: &mut HashMap<String, Module>, n: &Path) -> Result<Module, pest::error::Error<Rule>> {
    let mut module = Module::default();
    module.name = n.file_stem().expect(&format!("stem {:?}", n)).to_string_lossy().into();

    let mut f = std::fs::File::open(n).expect(&format!("cannot open file {:?}", n));
    let mut file = String::new();
    f.read_to_string(&mut file).expect(&format!("read {:?}", n));
    let mut file = ZZParser::parse(Rule::file, &file)?;

    for decl in file.next().unwrap().into_inner() {
        match decl.as_rule() {
            Rule::function => {
                let mut loc  = None;
                let decl = decl.into_inner();
                let mut name = String::new();
                let mut args = Vec::new();
                let mut ret  = None;
                let mut body = String::new();
                let mut vis = Visibility::Shared;

                for part in decl {
                    match part.as_rule() {
                        Rule::key_private => {
                            vis = Visibility::Object;
                        }
                        Rule::key_pub => {
                            vis = Visibility::Export;
                        }
                        Rule::ident => {
                            name = part.as_str().into();
                        }
                        Rule::ret_arg => {
                            ret = Some(AnonArg{
                                typ: part.into_inner().as_str().to_string()
                            });
                        },
                        Rule::fn_args => {
                            for arg in part.into_inner() {
                                let mut arg       = arg.into_inner();
                                let types         = arg.next().unwrap();
                                let name          = arg.next().unwrap().as_str().to_string();
                                let mut muta      = false;
                                let mut ptr       = false;
                                let mut typ       = String::new();
                                let mut namespace = None;

                                for part in types.into_inner() {
                                    match part.as_rule() {
                                        Rule::namespace => {
                                            namespace = Some(part.as_str().to_string());
                                        },
                                        Rule::key_ptr => {
                                            ptr = true;
                                        },
                                        Rule::ident => {
                                            typ = part.as_str().to_string();
                                        },
                                        Rule::key_const => {
                                            muta = false;
                                        },
                                        Rule::key_mut => {
                                            muta = true;
                                        },
                                        e => panic!("unexpected rule {:?} in function argument", e),
                                    }
                                }

                                args.push(NamedArg{
                                    name,
                                    typ,
                                    muta,
                                    ptr,
                                    namespace,
                                });
                            }
                        },
                        Rule::block => {
                            loc = Some(Location{line: part.as_span().start_pos().line_col().0, file: n.to_string_lossy().into()});
                            body = part.as_str().to_string();
                        },
                        e => panic!("unexpected rule {:?} in function", e),
                    }
                }

                module.functions.insert(name.clone(), Function{
                    name,
                    ret,
                    args,
                    body,
                    vis,
                    loc: loc.unwrap(),
                });
            },
            Rule::EOI => {},
            Rule::struct_d => {
                let decl = decl.into_inner();

                let mut vis   = Visibility::Shared;
                let mut name  = None;
                let mut body  = None;

                let mut loc   = None;

                for part in decl {
                    match part.as_rule() {
                        Rule::key_private => {
                            vis = Visibility::Object;
                        }
                        Rule::key_pub => {
                            vis = Visibility::Export;
                        }
                        Rule::ident => {
                            name = Some(part.as_str().into());
                        }
                        Rule::struct_c => {
                            loc  = Some(Location{line: part.as_span().start_pos().line_col().0, file: n.to_string_lossy().into()});
                            body = Some(part.as_str().into());
                        }
                        e => panic!("unexpected rule {:?} in struct ", e),
                    }
                };



                module.structs.push(Struct {
                    name: name.unwrap(),
                    body: body.unwrap(),
                    vis,
                    loc: loc.unwrap(),
                });
            }
            Rule::import => {
                let loc  = Location{line: decl.as_span().start_pos().line_col().0, file: n.file_name().unwrap().to_string_lossy().into()};
                let decl = decl.into_inner().next().unwrap();
                let span = decl.as_span();

                let mut ns   = String::new();
                let mut name = String::new();

                for part in decl.into_inner() {
                    match part.as_rule() {
                        Rule::ident_or_star => {
                            name  = part.as_str().into();
                        }
                        Rule::namespace => {
                            ns = part.into_inner().next().unwrap().as_str().into();
                        }
                        e => panic!("unexpected rule {:?} in import ", e),
                    }
                }

                if ns.is_empty() {
                    ns   = name;
                    name = "*".into();
                }

                if !modules.contains_key(&ns) {
                    let mut n2 = Path::new("./src").join(&ns).with_extension("zz");

                    if n2.exists() {
                        parse(modules, &Path::new(&n2));
                        module.imports.push(Import{name, namespace: ns, loc});
                    } else {
                        n2 = Path::new("./src").join(&ns).with_extension("h");

                        if n2.exists() {
                            module.includes.push(format!("{:?}", n2))
                        } else {
                            let e = pest::error::Error::<Rule>::new_from_span(pest::error::ErrorVariant::CustomError {
                                message: format!("cannot find module"),
                            }, span);
                            eprintln!("{} : {}", loc.file, e);
                            std::process::exit(9);
                        }
                    }

                }
            },
            Rule::include => {
                let im = decl.into_inner().as_str();
                module.includes.push(im.to_string());
            },
            e => panic!("unexpected rule {:?} in file", e),

        }

    }

    Ok(module)
}
