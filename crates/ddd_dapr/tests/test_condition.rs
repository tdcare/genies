#![warn(non_snake_case)]
#[allow(non_snake_case)]

#[cfg(test)]
pub mod condition_tests {
    use serde::*;
    use serde_json::json;
    use ddd_dapr::condition::{ConditionTree, obj_test};
    #[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
    struct DrugDoctorAdviceEntity{

        // @ApiModelProperty(value = "主键")
        // @Id
        // @Column
        // private String id;
        id:Option<String>,

        // @ApiModelProperty(value = "his主键")
        // @Column(length = 64)
        // private String hisId;
        hisId:Option<String>,

        // @ApiModelProperty(value = "医嘱类型")
        // private Integer doctorAdviceType;
        doctorAdviceType:Option<i32>,
        // @ApiModelProperty(value = "是否长期医嘱(1:是,0:否)")
        // private Integer isLongTerm;
        isLongTerm:Option<i32>,
        // @ApiModelProperty(value = "执行频次(DoctorAdviceCycleEntity)")
        // private Integer adviceCycle;
        adviceCycle:Option<i32>,
        // @ApiModelProperty(value = "执行频次名称(DoctorAdviceCycleEntity)")
        // @Column(length = 63)
        // private String adviceCycleName;
        adviceCycleName:Option<String>,
        // @ApiModelProperty(value = "是否有效(1:是,0:否)")
        // private Integer valide;
        valide:Option<i32>,


        // @ApiModelProperty(value = "用法名称")
        // @Column(length = 63)
        // private String methodName;
        methodName:Option<String>,
        drugItems:Option<Vec<DrugItemEntity>>,
    }
    #[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
    struct DrugItemEntity{
        // @ApiModelProperty(value = "医嘱id")
        // @Column(length = 36)
        // private String drugDoctorAdviceId;
        drugDoctorAdviceId:Option<String>,
        // // @ManyToOne(cascade= CascadeType.ALL)
        // // @JoinColumn(name="drugDoctorAdviceId")
        // // private DrugDoctorAdviceModel drugDoctorAdviceModel;
        //
        // @ApiModelProperty(value = "药品id")
        // @Column(length = 128)
        // private String drugHisId;
        drugHisId:Option<String>,
        // @ApiModelProperty(value = "药品名称")
        // @Column(length = 128)
        // private String drugName;
        drugName:Option<String>,
        // @ApiModelProperty(value = "药品商品名称")
        // @Column(length = 128)
        // private String tradeName;
        tradeName:Option<String>,
        // @ApiModelProperty(value = "(废弃,用drugAmount)药品数量")
        // private Integer drugNumber;
        drugNumber:Option<i32>,
        // @ApiModelProperty(value = "剂量")
        // private Float dosage;
        dosage:Option<f32>,
        // @ApiModelProperty(value = "剂量比例倍数")
        // @Column(length = 15)
        // private String dosageTimes;
        dosageTimes:Option<String>,
        // @ApiModelProperty(value = "his剂量 数量单位")
        // @Column(length = 64)
        // private String hisDosage;
        hisDosage:Option<String>,
        // @ApiModelProperty(value = "剂量单位")
        // private Integer dosageUnit;
        dosageUnit:Option<i32>,
        // @ApiModelProperty(value = "单位name")
        // @Column(length = 64)
        // private String unitName;
        unitName:Option<String>,
        // @ApiModelProperty(value = "滴速")
        // private Integer speed;
        speed:Option<i32>,
        // @ApiModelProperty(value = "用法描述")
        // private String methodDescribe;
        methodDescribe:Option<String>,
        // @ApiModelProperty(value = "备注")
        // private String remark;
        remark:Option<String>,
        // @ApiModelProperty(value = "是否需要皮试(1:是)")
        // private Integer isNeedSkinTest;
        isNeedSkinTest:Option<i32>,
        // @ApiModelProperty(value = "是否成品药(1:是)")
        // private Integer isFinishedMedicines;
        isFinishedMedicines:Option<i32>,
        // @ApiModelProperty(value = "费用类别(A西药,B中药")
        // @Column(length = 15)
        // private String billType;
        billType:Option<String>,
        // @ApiModelProperty(value = "药品规格")
        // @Column(length = 63)
        // private String drugSpec;
        drugSpec:Option<String>,
        // @ApiModelProperty(value = "药品数量")
        // private Float drugAmount;
        drugAmount:Option<f32>,
        // @ApiModelProperty(value = "医生描述")
        // private String doctorDescription;
        doctorDescription:Option<String>,
        // @ApiModelProperty(value = "是否为过敏药物0,非过敏药，1过敏药，2疑似过敏药")
        // @Column(length = 2)
        // private Integer allergyDrug;
        allergyDrug:Option<i32>,
        // @ApiModelProperty(value = "过敏时效")
        // @Column(length = 36)
        // private String allergyPrescription;
        allergyPrescription:Option<String>,
        // @ApiModelProperty(value = "过敏药类型")
        // private String allergType;
        allergType:Option<String>,
        // @ApiModelProperty(value = "高危药物类型")
        // private String highRiskType;
        highRiskType:Option<String>,
        // @ApiModelProperty(value = "是否是高危药物")
        // @Column(length = 2)
        // private Integer isHighRisk;
        isHighRisk:Option<i32>,
        // @ApiModelProperty(value = "皮试结果。-1 弱阴性，0 阴性，1 阳性，2 强阳性")
        // private Integer skinTestResult;
        skinTestResult:Option<i32>,
        // @ApiModelProperty(value = "简称别名")
        // private String abstractName;
        abstractName:Option<String>,
    }


    #[test]
    fn test(){



        let json1=r#"
        {"o":"or","c":[{"o":"contain","p":"methodName","v":"静脉滴注"},{"o":"contain","p":"methodName","value":"静脉输液"},{"o":"contain","p":"methodName","value":"续静滴"},{"operator":"contain","propertyName":"methodName","value":"泵入"}]}
        "#;

        let tree:ConditionTree=serde_json::from_str(json1).unwrap();
        println!("{:?}",tree);
        let obj=DrugDoctorAdviceEntity{
            id: None,
            hisId: None,
            doctorAdviceType: Some(1),
            isLongTerm: Some(2),
            adviceCycle: None,
            adviceCycleName: None,
            valide: None,
            methodName: Some("静脉滴注".to_string()),
            drugItems: None
        };

        let obj_value=serde_json::to_value(&obj).unwrap();
        println!("给药方法:{}",obj_value["methodName"]);
        println!("doctorAdviceType:{}",obj_value["doctorAdviceType"]);


        let  is_pass=obj_test(&obj_value,&tree);
        println!("{:?}",is_pass);


    }
    #[test]
    fn test_obj_vec(){
        let json=r#"{"operator": "and","conditionTrees": [{"operator": "or","conditionTrees": [{"operator": "contain","propertyName": "methodName","value": "穴位注射"},{"operator": "contain","propertyName": "methodName","value": "动脉灌注"},{"operator": "contain","propertyName": "methodName","value": "宫颈注射"},{"operator": "contain","propertyName": "methodName","value": "肌肉注射"},{"operator": "contain","propertyName": "methodName","value": "静脉推注"},{"operator": "contain","propertyName": "methodName","value": "皮下注射"},{"operator": "contain","propertyName": "methodName","value": "局部注射"}]},{"operator": "arr_each_contain","propertyName": "drugItems.drugName","value": "复方双氯芬酸钠"}]}"#;
        let _json1=r#"{"operator": "and","conditionTrees": [{"operator": "or","conditionTrees": [{"operator": "contain","propertyName": "methodName","value": "穴位注射"},{"operator": "contain","propertyName": "methodName","value": "动脉灌注"},{"operator": "contain","propertyName": "methodName","value": "宫颈注射"},{"operator": "contain","propertyName": "methodName","value": "肌肉注射"},{"operator": "contain","propertyName": "methodName","value": "静脉推注"},{"operator": "contain","propertyName": "methodName","value": "皮下注射"},{"operator": "contain","propertyName": "methodName","value": "局部注射"}]},{"operator": "arr_size_=","propertyName": "drugItems","value": "1"}]}"#;

        let tree:ConditionTree=serde_json::from_str(json).unwrap();

        let drug_item1=DrugItemEntity{
            drugDoctorAdviceId: Some("1".to_string()),
            drugHisId: None,
            drugName: Some("复方双氯芬酸钠".to_string()),
            tradeName: None,
            drugNumber: None,
            dosage: None,
            dosageTimes: None,
            hisDosage: None,
            dosageUnit: None,
            unitName: None,
            speed: None,
            methodDescribe: None,
            remark: None,
            isNeedSkinTest: None,
            isFinishedMedicines: None,
            billType: None,
            drugSpec: None,
            drugAmount: None,
            doctorDescription: None,
            allergyDrug: None,
            allergyPrescription: None,
            allergType: None,
            highRiskType: None,
            isHighRisk: None,
            skinTestResult: None,
            abstractName: None
        };
        let drg_item2=DrugItemEntity{
            drugDoctorAdviceId: Some("2".to_string()),
            drugHisId: None,
            drugName: Some("复方双氯芬酸钠".to_string()),
            tradeName: None,
            drugNumber: None,
            dosage: None,
            dosageTimes: None,
            hisDosage: None,
            dosageUnit: None,
            unitName: None,
            speed: None,
            methodDescribe: None,
            remark: None,
            isNeedSkinTest: None,
            isFinishedMedicines: None,
            billType: None,
            drugSpec: None,
            drugAmount: None,
            doctorDescription: None,
            allergyDrug: None,
            allergyPrescription: None,
            allergType: None,
            highRiskType: None,
            isHighRisk: None,
            skinTestResult: None,
            abstractName: None
        };
        let obj=DrugDoctorAdviceEntity{
            id: None,
            hisId: None,
            doctorAdviceType: None,
            isLongTerm: None,
            adviceCycle: None,
            adviceCycleName: None,
            valide: None,
            methodName: Some("静脉推注".to_string()),
            drugItems: Some(vec![drug_item1,drg_item2])
        };
        let obj_value=serde_json::to_value(&obj).unwrap();
        let is_pass1=obj_test(&obj_value,&tree);

        println!("is_pass1:{}",is_pass1);
        assert_eq!(is_pass1,true);
    }

    #[test]
    fn test_vec_len(){
        let v_json=json!({
            "s":["s1","s2"]
        });
        let v=v_json["s"].clone();
        let v_arr=v.as_array().unwrap().clone();
        let json_value=json!(v_arr.len());
        println!("{:?}",json_value);
        println!("{}",json_value.is_number());
        println!("{}",json_value.is_u64());
        println!("{}",json_value.is_i64());
        println!("{}",json_value.as_i64().unwrap());
        assert_eq!(json_value.as_i64().unwrap(),2,"{}",json_value.as_i64().unwrap());
    }
    #[test]
    fn test_string(){
      let op_json="arr_exist_!contain";
        let op:&str=&op_json[10..op_json.len()];
        println!("操作符为：{}",op);
        assert_eq!(op,"!contain");
        let op_json="drugItems.drugName";
        let op_arrs:Vec<&str> =op_json.split(".").collect();
        println!("{:?}",op_arrs);
        assert_eq!(op_arrs.len(),2);
        let pointer_string=format!("/{}/{}/{}",op_arrs[0],0,op_arrs[1]);
        println!("json pointer={}",pointer_string);
        assert_eq!(&pointer_string,"/drugItems/0/drugName");

        let property_name="drugItems.drugName";
        let pointer_string=format!("/{}",property_name).replace(".","/");
        println!("json pointer={}",pointer_string);
        assert_eq!(&pointer_string,"/drugItems/drugName");


    }
}