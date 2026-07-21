-- SickbedEntity 表
CREATE TABLE IF NOT EXISTS SickbedEntity (
    id VARCHAR(63) PRIMARY KEY,
    departmentId VARCHAR(255),
    departmentName VARCHAR(255),
    departmentCode VARCHAR(255),
    departmentAbstract VARCHAR(255),
    patientId VARCHAR(255),
    hisId VARCHAR(512),
    sickbedNo VARCHAR(32),
    wardId VARCHAR(255),
    wardName VARCHAR(36),
    status INT,
    effectiveness INT,
    bedLevel INT,
    nurseUserId VARCHAR(36),
    doctorUserId VARCHAR(36),
    doctorUserName VARCHAR(255),
    responseUserId VARCHAR(36),
    enterUserId VARCHAR(36),
    packetBedName VARCHAR(32),
    createDate DATETIME,
    wardCode VARCHAR(16),
    approvedType VARCHAR(255),
    sexLimit INT,
    sexType VARCHAR(4),
    bedClass VARCHAR(20),
    orderId INT,
    nurseUserName VARCHAR(63),
    INDEX idx_departmentId (departmentId),
    INDEX idx_departmentId_effectiveness (departmentId, effectiveness),
    INDEX idx_effectiveness (effectiveness)
);

-- WardEntity 表
CREATE TABLE IF NOT EXISTS WardEntity (
    id VARCHAR(63) PRIMARY KEY,
    departmentId VARCHAR(255),
    departmentName VARCHAR(255),
    departmentCode VARCHAR(255),
    departmentAbstract VARCHAR(255),
    hisId VARCHAR(512),
    wardNo VARCHAR(32),
    wardName VARCHAR(32),
    sickbedCount INT,
    predictLevel VARCHAR(255),
    address VARCHAR(128),
    effectiveness INT,
    responseUserId VARCHAR(36),
    responseUserName VARCHAR(36),
    enterUserId VARCHAR(36),
    createDate DATETIME,
    description VARCHAR(128),
    wardType VARCHAR(255),
    orderId INT
);

-- WardNoEntity 表（自增ID）
CREATE TABLE IF NOT EXISTS WardNoEntity (
    id INT AUTO_INCREMENT PRIMARY KEY,
    name VARCHAR(255),
    wardNo VARCHAR(255),
    type VARCHAR(255)
);
