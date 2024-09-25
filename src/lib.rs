#![allow(non_camel_case_types)]
#![deny(unsafe_op_in_unsafe_fn)]

use std::collections::HashMap;
use std::ops::Deref;
use std::path::Path;

use biodivine_lib_bdd::Bdd;
use biodivine_lib_bdd::BddVariable;
use biodivine_lib_bdd::BddVariableSet;

// Utils
#[repr(i8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum OptBool {
    /// Don't care
    None = -1,
    False = 0,
    True = 1,
}

/// Boolean assignment
///
/// `data` is a pointer to `len` values. A value can be either 0 (false), 1
/// (true), or -1 (don't care).
#[repr(C)]
pub struct bdd_assignment_t {
    data: *mut i8,
    len: usize,
}

/// Free the given assignment
///
/// To uphold Rust's invariants, all values in the assignment must be 0, 1, or
/// -1.
#[no_mangle]
pub unsafe extern "C" fn bdd_assignment_free(assignment: bdd_assignment_t) {
    if !assignment.data.is_null() {
        drop(unsafe {
            Vec::from_raw_parts(
                assignment.data as *mut OptBool,
                assignment.len,
                assignment.len,
            )
        })
    }
}

// BDD manaager & BDD functions

struct Manager {
    var_set: BddVariableSet,
    rc: usize,
    nodes_total: usize,
    max_nodes_total: usize,
}

impl Manager {
    fn new(var_set: BddVariableSet, max_nodes_total: usize) -> Self {
        Self {
            var_set,
            rc: 1,
            nodes_total: 0,
            max_nodes_total,
        }
    }
}

impl Deref for Manager {
    type Target = BddVariableSet;

    fn deref(&self) -> &BddVariableSet {
        &self.var_set
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct manager_t {
    _p: *mut Manager,
}

struct RcBdd {
    bdd: Bdd,
    rc: usize,
    manager: *mut Manager,
}

impl RcBdd {
    fn new(bdd: Bdd, manager: *mut Manager) -> Self {
        Self {
            bdd,
            rc: 1,
            manager,
        }
    }
}

impl Deref for RcBdd {
    type Target = Bdd;

    fn deref(&self) -> &Bdd {
        &self.bdd
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct VarPair {
    first: u16,
    second: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct bdd_t {
    _p: *mut RcBdd,
}

impl bdd_t {
    unsafe fn from_bdd(bdd: Bdd, manager: *mut Manager) -> Self {
        let m = unsafe { &mut *manager };
        m.rc += 1;
        m.nodes_total += bdd.size();
        if m.nodes_total > m.max_nodes_total {
            eprintln!("Too many nodes ({} > {})", m.nodes_total, m.max_nodes_total);
            std::process::abort();
        }
        bdd_t {
            _p: Box::into_raw(Box::new(RcBdd::new(bdd, manager))),
        }
    }
}

#[no_mangle]
pub extern "C" fn manager_new(num_vars: u16, max_nodes_total: usize) -> manager_t {
    let var_set = BddVariableSet::new_anonymous(num_vars);
    manager_t {
        _p: Box::into_raw(Box::new(Manager::new(var_set, max_nodes_total))),
    }
}

#[no_mangle]
pub unsafe extern "C" fn manager_ref(manager: manager_t) -> manager_t {
    unsafe { &mut *manager._p }.rc += 1;
    manager
}

#[no_mangle]
pub unsafe extern "C" fn manager_unref(manager: manager_t) {
    let rc = &mut unsafe { &mut *manager._p }.rc;
    if *rc == 1 {
        std::mem::drop(unsafe { Box::from_raw(manager._p) });
    } else {
        *rc -= 1;
    }
}

#[no_mangle]
pub unsafe extern "C" fn manager_node_count(manager: manager_t) -> usize {
    unsafe { &*manager._p }.nodes_total
}

#[no_mangle]
pub unsafe extern "C" fn manager_ithvar(manager: manager_t, i: u16) -> bdd_t {
    let bdd = unsafe { &*(manager._p) }.mk_var(BddVariable::from_index(i as usize));
    unsafe { bdd_t::from_bdd(bdd, manager._p) }
}

#[no_mangle]
pub unsafe extern "C" fn manager_nithvar(manager: manager_t, i: u16) -> bdd_t {
    let bdd = unsafe { &*(manager._p) }.mk_not_var(BddVariable::from_index(i as usize));
    unsafe { bdd_t::from_bdd(bdd, manager._p) }
}

#[no_mangle]
pub unsafe extern "C" fn manager_true(manager: manager_t) -> bdd_t {
    let bdd = unsafe { &*(manager._p) }.mk_true();
    unsafe { bdd_t::from_bdd(bdd, manager._p) }
}

#[no_mangle]
pub unsafe extern "C" fn manager_false(manager: manager_t) -> bdd_t {
    let bdd = unsafe { &*(manager._p) }.mk_false();
    unsafe { bdd_t::from_bdd(bdd, manager._p) }
}

#[no_mangle]
pub unsafe extern "C" fn bdd_ref(f: bdd_t) -> bdd_t {
    unsafe { &mut *f._p }.rc += 1;
    f
}
#[no_mangle]
pub unsafe extern "C" fn bdd_unref(f: bdd_t) {
    let bdd = unsafe { &mut *f._p };
    if bdd.rc == 1 {
        unsafe { &mut *bdd.manager }.nodes_total -= bdd.size();
        unsafe { manager_unref(manager_t { _p: bdd.manager }) };
        drop(unsafe { Box::from_raw(f._p) });
    } else {
        bdd.rc -= 1;
    }
}

#[no_mangle]
pub unsafe extern "C" fn bdd_not(f: bdd_t) -> bdd_t {
    let f = unsafe { &*f._p };
    let bdd = f.not();
    unsafe { bdd_t::from_bdd(bdd, f.manager) }
}

#[no_mangle]
pub unsafe extern "C" fn bdd_and(f: bdd_t, g: bdd_t) -> bdd_t {
    let f = unsafe { &*f._p };
    let g = unsafe { &*g._p };
    let bdd = f.and(g);
    unsafe { bdd_t::from_bdd(bdd, f.manager) }
}

#[no_mangle]
pub unsafe extern "C" fn bdd_or(f: bdd_t, g: bdd_t) -> bdd_t {
    let f = unsafe { &*f._p };
    let g = unsafe { &*g._p };
    let bdd = f.or(g);
    unsafe { bdd_t::from_bdd(bdd, f.manager) }
}

#[no_mangle]
pub unsafe extern "C" fn bdd_xor(f: bdd_t, g: bdd_t) -> bdd_t {
    let f = unsafe { &*f._p };
    let g = unsafe { &*g._p };
    let bdd = f.xor(g);
    unsafe { bdd_t::from_bdd(bdd, f.manager) }
}

#[no_mangle]
pub unsafe extern "C" fn bdd_imp(f: bdd_t, g: bdd_t) -> bdd_t {
    let f = unsafe { &*f._p };
    let g = unsafe { &*g._p };
    let bdd = f.imp(g);
    unsafe { bdd_t::from_bdd(bdd, f.manager) }
}

#[no_mangle]
pub unsafe extern "C" fn bdd_iff(f: bdd_t, g: bdd_t) -> bdd_t {
    let f = unsafe { &*f._p };
    let g = unsafe { &*g._p };
    let bdd = f.iff(g);
    unsafe { bdd_t::from_bdd(bdd, f.manager) }
}

#[no_mangle]
pub unsafe extern "C" fn bdd_and_not(f: bdd_t, g: bdd_t) -> bdd_t {
    let f = unsafe { &*f._p };
    let g = unsafe { &*g._p };
    let bdd = f.and_not(g);
    unsafe { bdd_t::from_bdd(bdd, f.manager) }
}

#[no_mangle]
pub unsafe extern "C" fn bdd_ite(f: bdd_t, g: bdd_t, h: bdd_t) -> bdd_t {
    let f = unsafe { &*f._p };
    let g = unsafe { &*g._p };
    let h = unsafe { &*h._p };
    let bdd = Bdd::if_then_else(f, g, h);
    unsafe { bdd_t::from_bdd(bdd, f.manager) }
}

#[no_mangle]
pub unsafe extern "C" fn bdd_var_exists(f: bdd_t, var: u16) -> bdd_t {
    let f = unsafe { &*f._p };
    let bdd = f.var_exists(BddVariable::from_index(var as usize));
    unsafe { bdd_t::from_bdd(bdd, f.manager) }
}

#[no_mangle]
pub unsafe extern "C" fn bdd_var_forall(f: bdd_t, var: u16) -> bdd_t {
    let f = unsafe { &*f._p };
    let bdd = f.var_for_all(BddVariable::from_index(var as usize));
    unsafe { bdd_t::from_bdd(bdd, f.manager) }
}

#[no_mangle]
pub unsafe extern "C" fn bdd_exists(f: bdd_t, vars: *const u16, num_vars: usize) -> bdd_t {
    let f = unsafe { &*f._p };
    let vars: Vec<BddVariable> = unsafe { &*std::ptr::slice_from_raw_parts(vars, num_vars) }
        .iter()
        .map(|&v| BddVariable::from_index(v as usize))
        .collect();
    let bdd = f.exists(&vars);
    unsafe { bdd_t::from_bdd(bdd, f.manager) }
}

#[no_mangle]
pub unsafe extern "C" fn bdd_forall(f: bdd_t, vars: *const u16, num_vars: usize) -> bdd_t {
    let f = unsafe { &*f._p };
    let vars: Vec<BddVariable> = unsafe { &*std::ptr::slice_from_raw_parts(vars, num_vars) }
        .iter()
        .map(|&v| BddVariable::from_index(v as usize))
        .collect();
    let bdd = f.for_all(&vars);
    unsafe { bdd_t::from_bdd(bdd, f.manager) }
}

#[no_mangle]
pub unsafe extern "C" fn bdd_and_exists(
    f: bdd_t,
    g: bdd_t,
    vars: *const u16,
    num_vars: usize,
) -> bdd_t {
    let f = unsafe { &*f._p };
    let g = unsafe { &*g._p };
    let vars: Vec<BddVariable> = unsafe { &*std::ptr::slice_from_raw_parts(vars, num_vars) }
        .iter()
        .map(|&v| BddVariable::from_index(v as usize))
        .collect();
    let bdd = Bdd::binary_op_with_exists(&f, &g, biodivine_lib_bdd::op_function::and, &vars);
    unsafe { bdd_t::from_bdd(bdd, f.manager) }
}

#[no_mangle]
pub unsafe extern "C" fn bdd_or_exists(
    f: bdd_t,
    g: bdd_t,
    vars: *const u16,
    num_vars: usize,
) -> bdd_t {
    let f = unsafe { &*f._p };
    let g = unsafe { &*g._p };
    let vars: Vec<BddVariable> = unsafe { &*std::ptr::slice_from_raw_parts(vars, num_vars) }
        .iter()
        .map(|&v| BddVariable::from_index(v as usize))
        .collect();
    let bdd = Bdd::binary_op_with_exists(&f, &g, biodivine_lib_bdd::op_function::or, &vars);
    unsafe { bdd_t::from_bdd(bdd, f.manager) }
}

#[no_mangle]
pub unsafe extern "C" fn bdd_and_forall(
    f: bdd_t,
    g: bdd_t,
    vars: *const u16,
    num_vars: usize,
) -> bdd_t {
    let f = unsafe { &*f._p };
    let g = unsafe { &*g._p };
    let vars: Vec<BddVariable> = unsafe { &*std::ptr::slice_from_raw_parts(vars, num_vars) }
        .iter()
        .map(|&v| BddVariable::from_index(v as usize))
        .collect();
    let bdd = Bdd::binary_op_with_for_all(&f, &g, biodivine_lib_bdd::op_function::and, &vars);
    unsafe { bdd_t::from_bdd(bdd, f.manager) }
}

#[no_mangle]
pub unsafe extern "C" fn bdd_or_forall(
    f: bdd_t,
    g: bdd_t,
    vars: *const u16,
    num_vars: usize,
) -> bdd_t {
    let f = unsafe { &*f._p };
    let g = unsafe { &*g._p };
    let vars: Vec<BddVariable> = unsafe { &*std::ptr::slice_from_raw_parts(vars, num_vars) }
        .iter()
        .map(|&v| BddVariable::from_index(v as usize))
        .collect();
    let bdd = Bdd::binary_op_with_for_all(&f, &g, biodivine_lib_bdd::op_function::or, &vars);
    unsafe { bdd_t::from_bdd(bdd, f.manager) }
}

#[no_mangle]
pub unsafe extern "C" fn bdd_rename_variable(f: bdd_t, x: u16, y: u16) -> bdd_t {
    let f = unsafe { &*f._p };
    let mut g = f.bdd.clone();
    unsafe {
        g.rename_variable(
            BddVariable::from_index(x as usize),
            BddVariable::from_index(y as usize),
        )
    };
    unsafe { bdd_t::from_bdd(g, f.manager) }
}

#[no_mangle]
pub unsafe extern "C" fn bdd_rename_variables(
    f: bdd_t,
    var_pairs: *const VarPair,
    num_pairs: usize,
) -> bdd_t {
    let f = unsafe { &*f._p };
    let var_map: HashMap<BddVariable, BddVariable> =
        unsafe { &*std::ptr::slice_from_raw_parts(var_pairs, num_pairs) }
            .iter()
            .map(|p| {
                (
                    BddVariable::from_index(p.first as usize),
                    BddVariable::from_index(p.second as usize),
                )
            })
            .collect();
    let mut g = f.bdd.clone();
    unsafe { g.rename_variables(&var_map) };
    unsafe { bdd_t::from_bdd(g, f.manager) }
}

#[no_mangle]
pub unsafe extern "C" fn bdd_nodecount(f: bdd_t) -> usize {
    unsafe { &**f._p }.size()
}

#[no_mangle]
pub unsafe extern "C" fn bdd_satcount(f: bdd_t) -> f64 {
    unsafe { &**f._p }.cardinality()
}

#[no_mangle]
pub unsafe extern "C" fn bdd_eq(f: bdd_t, g: bdd_t) -> bool {
    let f = unsafe { &**f._p };
    let g = unsafe { &**g._p };
    f == g
}

#[no_mangle]
pub unsafe extern "C" fn bdd_pickcube(f: bdd_t) -> bdd_assignment_t {
    let f = unsafe { &**f._p };
    if f.is_false() {
        return bdd_assignment_t {
            data: std::ptr::null_mut(),
            len: 0,
        };
    }
    let mut assignment = vec![OptBool::None; f.num_vars() as usize];
    let mut p = f.root_pointer();
    while !p.is_one() {
        let c = f.low_link_of(p);
        if !c.is_zero() {
            assignment[f.var_of(p).to_index()] = OptBool::False;
            p = c;
        } else {
            let c = f.high_link_of(p);
            debug_assert!(!c.is_zero());
            assignment[f.var_of(p).to_index()] = OptBool::True;
            p = c;
        }
    }

    assignment.shrink_to_fit();
    let len = assignment.len();
    let data = assignment.as_mut_ptr() as *mut i8;
    std::mem::forget(assignment);
    bdd_assignment_t { data, len }
}

#[no_mangle]
pub unsafe extern "C" fn bdd_save(f: bdd_t, path: *const std::ffi::c_char) -> () {
    let f = unsafe { &**f._p };
    let f_bytes = f.to_bytes();

    let path_cstr = unsafe { std::ffi::CStr::from_ptr(path) };
    std::fs::write(Path::new(path_cstr.to_str().unwrap()), f_bytes).unwrap();
}
