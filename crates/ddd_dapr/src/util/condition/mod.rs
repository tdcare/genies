#![warn(unused_assignments)]
#![warn(non_snake_case)]

use serde::{Deserialize, Serialize};
use serde_json::*;


#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ConditionTree{
    #[serde(alias = "o")]
    pub operator:Option<String>,
    #[serde(alias = "p")]
    pub propertyName:Option<String>,
    #[serde(alias = "v")]
    pub value:Option<String>,
    #[serde(alias = "c")]
    pub conditionTrees:Option<Vec<ConditionTree>>,
}

/// 测试对象是否满足 ConditionTree 描述的条件:
/// 参数： obj_value 对象序列化为json Value
/// 参数：tree 条件表达式
pub fn obj_test(obj_value:&Value,tree:&ConditionTree)->bool{
    let mut  result=true;

    if obj_value.is_null(){
       return false
   }
   let  operator= match tree.operator.clone() {
        Some(op)=>{
            op.to_lowercase()
        },
        None=>{
            return true
        }
    };

   if operator.eq("and"){
       result=operator_and(obj_value,tree);
   }else if operator.eq("or") {
       result=operator_or(obj_value,tree);
   }else {
       result=compare(obj_value,tree,&operator);
   }
    return result;
}


fn operator_and(obj_value:&Value,tree:&ConditionTree) ->bool{
    let mut result = true;
    let brach=tree.conditionTrees.clone().unwrap_or_default();
    for t in brach{
        if !obj_test(obj_value,&t){
           result=false;
            break;
        }
    }
   return result;
}
fn operator_or(obj_value:&Value,tree:&ConditionTree) ->bool{
    let mut result = false;
    let brach=tree.conditionTrees.clone().unwrap_or_default();
    for t in brach{
        if obj_test(obj_value,&t){
            result=true;
            break;
        }
    }
    return result;
}
fn compare(obj_value:&Value,tree:&ConditionTree,operator:&str)->bool{
    let mut result = false;
    let property_name =tree.propertyName.clone().unwrap();
    let tree_value=tree.value.clone().unwrap();
    // 字段是否是数组
    if !operator.starts_with("arr_") {
        let property_value =obj_value[&property_name].clone();
        result = compare_left_right(operator, &property_value, &tree_value);
    }else {
        // 如果对象属性为 数组
        let mut property_value_vec:Vec<Value>=vec![];
        let property_names:Vec<&str>=property_name.split(".").collect();
        let property_value =obj_value[&property_names[0]].clone();

        if property_value.is_array(){
          property_value_vec= property_value.as_array().unwrap().clone();
      }

        if operator.starts_with("arr_size_"){
             let obj_p=json!(property_value_vec.len());
             let op=&operator[9..operator.len()];
             result=compare_left_right(&op,&obj_p,&tree_value)
        }else if property_value_vec.len()>0 {
            if operator.starts_with("arr_exist_") {
                for i in 0..property_value_vec.len()-1 {
                    let pointer_string = format!("/{}/{}/{}", property_names[0], i, property_names[1]);
                    let obj_p=obj_value.pointer(&pointer_string).unwrap();
                    let op=&operator[10..operator.len()];
                    if compare_left_right(&op, obj_p, &tree_value) {
                        result=true;
                        break
                    }
                }
            }else if operator.starts_with("arr_each_") {
                result=true;
                for i in 0..property_value_vec.len()-1 {
                    let pointer_string = format!("/{}/{}/{}", property_names[0], i, property_names[1]);
                    let obj_p=obj_value.pointer(&pointer_string).unwrap();
                    let op=&operator[9..operator.len()];
                    if !compare_left_right(&op, obj_p, &tree_value) {
                        result=false;
                        break
                    }
                }
            }


        }
    }

    return result;
}
fn compare_left_right( operator:&str,   left:&Value,  right:&str)->bool{
   let mut result=false;
    if left.is_string(){
       let left_val=left.as_str().unwrap();
       let right_val=right;
       match operator.clone() {
           "="=>{
               result=left_val.eq(right_val);
           },
           "<>"=>{
               result=!left_val.eq(right_val);
           },
           "!="=>{
               result=!left_val.eq(right_val);
           },
           "contain"=>{
             result= left_val.contains(right_val);
           }
           "!contain"=>{
               result= !left_val.contains(right_val);
           }
           _ => {}
       }
   }
    if left.is_i64(){
       let left_val=left.as_i64().unwrap();
       let right_val=right.parse::<i64>().unwrap();
       match operator.clone() {
           "<"=>{
               result=left_val<right_val;
           },
           "<="=>{
               result=left_val<=right_val;
           },
           "="=>{
               result=left_val==right_val;
           },
           ">"=>{
               result= left_val>right_val;
           },
           ">="=>{
               result= left_val>=right_val;
           },
           _ => {}
       }
   }
    if left.is_u64(){
        let left_val=left.as_u64().unwrap();
        let right_val=right.parse::<u64>().unwrap();
        match operator.clone() {
            "<"=>{
                result=left_val<right_val;
            },
            "<="=>{
                result=left_val<=right_val;
            },
            "="=>{
                result=left_val==right_val;
            },
            ">"=>{
                result= left_val>right_val;
            },
            ">="=>{
                result= left_val>=right_val;
            },
            _ => {}
        }
    }
    if left.is_f64(){
        let left_val=left.as_f64().unwrap();
        let right_val=right.parse::<f64>().unwrap();
        match operator.clone() {
            "<"=>{
                result=left_val<right_val;
            },
            "<="=>{
                result=left_val<=right_val;
            },
            "="=>{
                result=left_val==right_val;
            },
            ">"=>{
                result= left_val>right_val;
            },
            ">="=>{
                result= left_val>=right_val;
            },
            _ => {}
        }
    }


    return result;
}


