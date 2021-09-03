//! This module includes interfaces about course, major and score.

use actix_web::{get, web, HttpResponse};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::bridge::{
    HostError, MajorRequest, RequestFrame, RequestPayload, ResponsePayload, SaveScScore,
    ScScoreItemRequest, SchoolYear, ScoreRequest, Semester, TimeTableRequest,
};
use crate::error::{ApiError, Result};
use crate::models::edu::{self, query_current_sc_score_list, save_sc_score_list, AvailClassroomQuery};
use crate::models::user::Person;
use crate::models::{CommonError, PageView};
use crate::services::response::ApiResponse;
use crate::services::{AppState, JwtToken};

const CAMPUS_XUHUI: i32 = 2;
const CAMPUS_FENGXIAN: i32 = 1;

#[derive(serde::Deserialize, sqlx::FromRow)]
pub struct ClassroomQuery {
    pub building: Option<String>,
    pub region: Option<String>,
    pub campus: Option<i32>,
    pub date: String,
    pub time: Option<String>,
}

#[get("/edu/classroom/available")]
pub async fn query_available_classrooms(
    app: web::Data<AppState>,
    query: web::Query<ClassroomQuery>,
    page: web::Query<PageView>,
) -> Result<HttpResponse> {
    let query = query.into_inner();

    let want_time = query.time.unwrap_or_else(|| String::from("0-0"));
    // See: model::edu::classroom::AvailClassroomQuery::want_time
    let want_time_bits = edu::convert_time_string(&want_time);

    let campus = query.campus.unwrap_or(0);
    // Judge the campus weather it is true number
    if campus != CAMPUS_FENGXIAN && campus != CAMPUS_XUHUI {
        return Err(ApiError::new(CommonError::Parameter));
    }

    let (term_week, week_day) = edu::transform_date(&query.date);
    let query = AvailClassroomQuery {
        building: query.building,
        region: query.region,
        campus: Some(campus),
        week: term_week,
        day: week_day,
        want_time: Some(want_time_bits),
    };
    let result = edu::query_avail_classroom(&app.pool, &query, &page).await?;
    let response = serde_json::json!({
        "rooms": result,
    });
    Ok(HttpResponse::Ok().json(&ApiResponse::normal(response)))
}

#[derive(Debug, Deserialize)]
pub struct TimeTableQuery {
    pub year: String,
    // "2021-2022"
    pub semester: i32,
}

#[get("/edu/timetable")]
pub async fn query_timetable(
    token: Option<JwtToken>,
    app: web::Data<AppState>,
    params: web::Query<TimeTableQuery>,
) -> Result<HttpResponse> {
    let params = params.into_inner();
    let first_year = params
        .year
        .split_once("-")
        .and_then(|(first, _)| {
            let year = first.parse::<i32>().unwrap_or_default();
            if (2015..2030).contains(&year) {
                Some(year)
            } else {
                None
            }
        })
        .ok_or_else(|| ApiError::new(CommonError::Parameter))?;

    let year = SchoolYear::SomeYear(first_year);
    let semester = match params.semester {
        1 => Semester::FirstTerm,
        2 => Semester::SecondTerm,
        _ => Semester::All,
    };
    let uid = token
        .ok_or_else(|| ApiError::new(CommonError::LoginNeeded))
        .map(|token| token.uid)?;
    let identity = Person::get_identity(&app.pool, uid)
        .await?
        .ok_or_else(|| ApiError::new(CommonError::IdentityNeeded))?;

    let data = TimeTableRequest {
        account: identity.student_id,
        passwd: identity.oa_secret,
        school_year: year,
        semester,
    };

    let agents = &app.agents;
    let request = RequestFrame::new(RequestPayload::TimeTable(data));
    let response = agents.request(request).await??;
    if let ResponsePayload::TimeTable(timetable) = response {
        let response = json!({
            "timeTable": timetable,
        });
        Ok(HttpResponse::Ok().json(ApiResponse::normal(response)))
    } else {
        Err(ApiError::new(HostError::Mismatched))
    }
}

#[get("/edu/timetable/ics")]
pub async fn export_timetable(
    token: Option<JwtToken>,
    app: web::Data<AppState>,
    params: web::Query<TimeTableQuery>,
) -> Result<HttpResponse> {
    let params = params.into_inner();
    let first_year = params
        .year
        .split_once("-")
        .and_then(|(first, _)| {
            let year = first.parse::<i32>().unwrap_or_default();
            if (2015..2030).contains(&year) {
                Some(year)
            } else {
                None
            }
        })
        .ok_or_else(|| ApiError::new(CommonError::Parameter))?;

    let year = SchoolYear::SomeYear(first_year);
    let semester = match params.semester {
        1 => Semester::FirstTerm,
        2 => Semester::SecondTerm,
        _ => Semester::All,
    };
    let uid = token
        .ok_or_else(|| ApiError::new(CommonError::LoginNeeded))
        .map(|token| token.uid)?;
    let identity = Person::get_identity(&app.pool, uid)
        .await?
        .ok_or_else(|| ApiError::new(CommonError::IdentityNeeded))?;

    let data = TimeTableRequest {
        account: identity.student_id,
        passwd: identity.oa_secret,
        school_year: year,
        semester,
    };

    let agents = &app.agents;
    let request = RequestFrame::new(RequestPayload::TimeTable(data));
    let response = agents.request(request).await??;
    if let ResponsePayload::TimeTable(timetable) = response {
        let calendar_text = edu::export_course_list_to_calendar(&timetable);

        Ok(HttpResponse::Ok()
            .content_type("text/calendar")
            .body(calendar_text))
    } else {
        Err(ApiError::new(HostError::Mismatched))
    }
}

#[derive(Debug, Deserialize)]
pub struct ScoreQuery {
    pub year: Option<i32>,
    pub semester: i32,
}

#[get("/edu/score")]
pub async fn query_score(
    token: Option<JwtToken>,
    app: web::Data<AppState>,
    params: web::Query<ScoreQuery>,
) -> Result<HttpResponse> {
    let params = params.into_inner();

    let year = match params.year {
        Some(y) => SchoolYear::SomeYear(y),
        None => SchoolYear::AllYear,
    };

    let semester = match params.semester {
        1 => Semester::FirstTerm,
        2 => Semester::SecondTerm,
        _ => Semester::All,
    };
    let uid = token
        .ok_or_else(|| ApiError::new(CommonError::LoginNeeded))
        .map(|token| token.uid)?;
    let identity = Person::get_identity(&app.pool, uid)
        .await?
        .ok_or_else(|| ApiError::new(CommonError::IdentityNeeded))?;
    let data = ScoreRequest {
        account: identity.student_id,
        passwd: identity.oa_secret,
        school_year: year,
        semester,
    };

    let agents = &app.agents;
    let request = RequestFrame::new(RequestPayload::Score(data));
    let response = agents.request(request).await??;
    if let ResponsePayload::Score(score) = response {
        let response = json!({
            "score": score,
        });
        Ok(HttpResponse::Ok().json(ApiResponse::normal(response)))
    } else {
        Err(ApiError::new(HostError::Mismatched))
    }
}

#[get("/edu/calendar")]
pub async fn get_school_start_date() -> Result<HttpResponse> {
    use chrono::NaiveDate;

    let date = NaiveDate::from_ymd(2021, 9, 6);
    let response = json!({
        "year": "2021-2022",
        "semester": 1,
        "start": date,
    });
    Ok(HttpResponse::Ok().json(ApiResponse::normal(response)))
}

#[get("/edu/schedule")]
pub async fn get_school_schedule() -> Result<HttpResponse> {
    let response = json!({
       "奉贤校区": {
            "default": [
                ["8:20","9:05"],
                ["9:10","9:55"],
                ["10:15","11:00"],
                ["11:05","11:50"],
                ["13:00","13:45"],
                ["13:50","14:35"],
                ["14:55","15:40"],
                ["15:45","16:30"],
                ["18:00","18:45"],
                ["18:50","19:35"],
                ["19:40","20:25"],
            ],
            "一教": [
                ["8:20","9:05"],
                ["9:10","9:55"],
                ["10:25","11:00"],
                ["11:05","12:00"],
                ["13:00","13:45"],
                ["13:50","14:35"],
                ["14:55","15:40"],
                ["15:45","16:30"],
                ["18:00","18:45"],
                ["18:50","19:35"],
                ["19:40","20:25"]
            ],
            "二教": [
                ["8:20","9:05"],
                ["9:10","9:55"],
                ["10:15","11:00"],
                ["11:05","11:45"],
                ["13:00","13:45"],
                ["13:50","14:35"],
                ["14:55","15:40"],
                ["15:45","16:30"],
                ["18:00","18:45"],
                ["18:50","19:35"],
                ["19:40","20:25"]
            ],
        },
        "徐汇校区": {
            "default": [
                ["8:00","8:45"],
                ["8:50","9:35"],
                ["9:55","10:40"],
                ["10:45","11:30"],
                ["13:00","13:45"],
                ["13:50","14:35"],
                ["14:55","15:40"],
                ["15:45","16:30"],
                ["18:00","18:45"],
                ["18:50","19:35"],
                ["19:40","20:25"]
            ],
        }
    });
    Ok(HttpResponse::Ok().json(ApiResponse::normal(response)))
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MajorQuery {
    pub entrance_year: Option<i32>,
    pub account: String,
    pub passwd: String,
}

#[get("/edu/major")]
pub async fn get_major_list(
    params: web::Query<MajorQuery>,
    app: web::Data<AppState>,
) -> Result<HttpResponse> {
    let params = params.into_inner();
    let year = match params.entrance_year {
        Some(year) => SchoolYear::SomeYear(year),
        None => SchoolYear::AllYear,
    };

    // TODO: Use cached majorList by default, or use random account to fetch in agent.
    let data = MajorRequest {
        entrance_year: year,
        account: params.account,
        passwd: params.passwd,
    };
    let agents = &app.agents;

    let request = RequestFrame::new(RequestPayload::MajorList(data));
    let response = agents.request(request).await??;
    if let ResponsePayload::MajorList(major_list) = response {
        let response = json!({
            "majorList": major_list,
        });
        Ok(HttpResponse::Ok().json(ApiResponse::normal(response)))
    } else {
        Err(ApiError::new(HostError::Mismatched))
    }
}

pub async fn query_sc_score_list(
    token: Option<JwtToken>,
    app: web::Data<AppState>,
    force: bool,
) -> Result<()> {
    if force {
        let uid = token
            .ok_or_else(|| ApiError::new(CommonError::LoginNeeded))
            .map(|token| token.uid)?;
        let identity = Person::get_identity(&app.pool, uid)
            .await?
            .ok_or_else(|| ApiError::new(CommonError::IdentityNeeded))?;

        let data = ScScoreItemRequest {
            account: identity.student_id,
            passwd: identity.oa_secret,
        };
        let student_id = (&data.account).to_string();
        let agent = &app.agents;
        let score_detail = query_current_sc_score_list(agent, data).await?;
        let save_detail: Vec<SaveScScore> = score_detail
            .into_iter()
            .map(|e| SaveScScore {
                account: student_id.clone(),
                activity_id: e.activity_id,
                amount: e.amount,
            })
            .collect();

        save_sc_score_list(&app.pool, save_detail);
        Ok(())
    } else {
        //todo get_sc_score_list
        Ok(())
    }
}
